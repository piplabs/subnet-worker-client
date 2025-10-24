#!/usr/bin/env python3
"""
REST API-based Worker Execution Process (WEP) server
"""
import asyncio
import os
import time
import base64
from typing import Dict, Optional
from contextlib import asynccontextmanager
import uvicorn
from fastapi import FastAPI, HTTPException, BackgroundTasks
from pydantic import BaseModel, Field, field_validator

# Task registry and handlers
from poseidon_wep_sdk_pkg.registry import task, bind_spec_from_yaml
from poseidon_wep_sdk_pkg.types import TaskAssignment as SDKTaskAssignment, Completion as SDKCompletion

# In-memory task storage (use Redis in production)
tasks: Dict[str, "TaskStatus"] = {}
running_tasks: Dict[str, asyncio.Task] = {}


class InputDescriptor(BaseModel):
    name: str
    media_type: str
    ref: str = ""
    inline_json: str = ""
    inline_bytes: str = ""  # Base64 encoded string
    
    @field_validator('inline_bytes')
    def validate_base64(cls, v):
        if v:
            try:
                base64.b64decode(v)
            except Exception:
                raise ValueError('inline_bytes must be valid base64')
        return v


class TaskAssignment(BaseModel):
    activity_id: str
    workflow_instance_id: str
    run_id: str
    task_kind: str
    task_version: str
    inputs: list[InputDescriptor]
    upload_prefix: str
    soft_deadline_unix: int = 0
    heartbeat_interval_s: int = 30


class TaskStatus(BaseModel):
    status: str = "pending"  # pending, running, completed, failed
    progress: int = Field(default=0, ge=0, le=100)
    result_ref: Optional[str] = None
    error: Optional[str] = None
    started_at: Optional[float] = None
    completed_at: Optional[float] = None


class TaskResponse(BaseModel):
    message: str
    task_id: str


# Bind workflow specs
base_dir = os.path.dirname(__file__)
spec_path = os.path.abspath(os.path.join(base_dir, "..", "shared", "workflows", "video_v1_0_0.yaml"))
try:
    bind_spec_from_yaml("video.preprocess", "1.0.0", spec_path)
    print(f"Bound spec from {spec_path}")
except Exception as e:
    print(f"Warning: Could not bind spec from {spec_path}: {e}")


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup
    print("WEP REST API starting up...")
    yield
    # Shutdown - cancel any running tasks
    print("WEP REST API shutting down...")
    for task_id, task in running_tasks.items():
        if not task.done():
            task.cancel()
    # Wait briefly for tasks to cancel
    await asyncio.sleep(0.5)


app = FastAPI(
    title="Worker Execution Process (WEP)",
    version="1.0.0",
    lifespan=lifespan
)


@app.post("/tasks/{task_id}/assign", status_code=202, response_model=TaskResponse)
async def assign_task(
    task_id: str,
    assignment: TaskAssignment,
    background_tasks: BackgroundTasks
):
    """Assign a new task to the WEP"""
    if task_id in tasks:
        raise HTTPException(status_code=409, detail=f"Task {task_id} already exists")
    
    # Validate task_id matches activity_id
    if task_id != assignment.activity_id:
        raise HTTPException(
            status_code=400, 
            detail=f"Task ID {task_id} does not match activity_id {assignment.activity_id}"
        )
    
    # Initialize task status
    tasks[task_id] = TaskStatus(status="pending")
    
    # Start processing in background
    task = asyncio.create_task(process_task(task_id, assignment))
    running_tasks[task_id] = task
    
    print(f"WEP: Accepted task {task_id} ({assignment.task_kind}:{assignment.task_version})")
    
    return TaskResponse(
        message="Task accepted for processing",
        task_id=task_id
    )


@app.get("/tasks/{task_id}/status", response_model=TaskStatus)
async def get_task_status(task_id: str):
    """Get the current status of a task"""
    if task_id not in tasks:
        raise HTTPException(status_code=404, detail=f"Task {task_id} not found")
    
    return tasks[task_id]


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    total_tasks = len(tasks)
    running = sum(1 for t in tasks.values() if t.status == "running")
    completed = sum(1 for t in tasks.values() if t.status == "completed")
    failed = sum(1 for t in tasks.values() if t.status == "failed")
    
    return {
        "status": "healthy",
        "tasks": {
            "total": total_tasks,
            "running": running,
            "completed": completed,
            "failed": failed
        }
    }


async def process_task(task_id: str, assignment: TaskAssignment):
    """Process a task asynchronously"""
    try:
        # Update status to running
        tasks[task_id].status = "running"
        tasks[task_id].started_at = time.time()
        
        print(f"WEP: Processing task {task_id}")
        
        # Convert to SDK types
        sdk_assignment = SDKTaskAssignment(
            activity_id=assignment.activity_id,
            workflow_instance_id=assignment.workflow_instance_id,
            run_id=assignment.run_id,
            task_kind=assignment.task_kind,
            task_version=assignment.task_version,
            inputs=[{
                "name": i.name,
                "media_type": i.media_type,
                "ref": i.ref,
                "inline_json": i.inline_json,
                "inline_bytes": base64.b64decode(i.inline_bytes) if i.inline_bytes else b""
            } for i in assignment.inputs],
            upload_prefix=assignment.upload_prefix,
            soft_deadline_unix=assignment.soft_deadline_unix,
            heartbeat_interval_s=assignment.heartbeat_interval_s
        )
        
        # Get handler from registry
        from poseidon_wep_sdk_pkg.registry import get_handler
        handler = get_handler(assignment.task_kind, assignment.task_version)
        if not handler:
            raise ValueError(f"No handler for {assignment.task_kind}:{assignment.task_version}")
        
        # Simulate progress updates during processing
        progress_task = asyncio.create_task(update_progress(task_id))
        
        try:
            # Run handler (blocking call in thread)
            result: SDKCompletion = await asyncio.to_thread(handler, sdk_assignment)
            
            # Update task status with result
            tasks[task_id].status = "completed"
            tasks[task_id].progress = 100
            tasks[task_id].result_ref = result.result_ref
            tasks[task_id].completed_at = time.time()
            
            print(f"WEP: Task {task_id} completed successfully")
            
        finally:
            progress_task.cancel()
            
    except Exception as e:
        # Update task status with error
        tasks[task_id].status = "failed"
        tasks[task_id].error = str(e)
        tasks[task_id].completed_at = time.time()
        
        print(f"WEP: Task {task_id} failed: {e}")
    
    finally:
        # Clean up running task reference
        running_tasks.pop(task_id, None)


async def update_progress(task_id: str):
    """Simulate progress updates during task processing"""
    try:
        for i in range(10):
            await asyncio.sleep(1)
            if tasks[task_id].status == "running":
                tasks[task_id].progress = (i + 1) * 10
    except asyncio.CancelledError:
        pass


# Import existing task handlers
@task("video.preprocess", "1.0.0")
def preprocess(assign: SDKTaskAssignment) -> SDKCompletion:
    """Example video preprocessing handler"""
    print(f"preprocessing {assign.activity_id}")
    
    # Simulate processing time
    time.sleep(10)
    
    return SDKCompletion(
        activity_id=assign.activity_id,
        run_id=assign.run_id,
        status="SUCCESS",
        result_ref=f"{assign.upload_prefix}/preprocess/result.json"
    )


if __name__ == "__main__":
    host = os.environ.get("WEP_HOST", "127.0.0.1")
    port = int(os.environ.get("WEP_PORT", "8080"))
    
    print(f"Starting WEP REST API server on {host}:{port}")
    
    uvicorn.run(
        app,
        host=host,
        port=port,
        log_level="info"
    )
