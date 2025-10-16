fn main() {
    let proto = "../../proto/execution/v1/execution.proto";
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&[proto], &["../../proto"])
        .expect("failed to compile proto");
    println!("cargo:rerun-if-changed={}", proto);
}
