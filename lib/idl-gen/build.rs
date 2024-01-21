fn main() {
    volo_build::ConfigBuilder::default().write().unwrap();

    println!("cargo:rerun-if-changed=../../idl/bss_websocket_client.proto");
    println!("cargo:rerun-if-changed=../../idl/game_data/bss_heartstone.proto");

    protobuf_codegen::Codegen::new()
        .pure()
        .includes(["../../idl"])
        .input("../../idl/bss_websocket_client.proto")
        .input("../../idl/game_data/bss_heartstone.proto")
        .cargo_out_dir("protos")
        .run_from_script();
}
