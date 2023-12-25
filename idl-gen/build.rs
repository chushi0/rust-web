fn main() {
    volo_build::ConfigBuilder::default().write().unwrap();

    protobuf_codegen::Codegen::new()
        .pure()
        .includes(&["../idl"])
        .input("../idl/bss_websocket_client.proto")
        .input("../idl/game_data/bss_heartstone.proto")
        .cargo_out_dir("protos")
        .run_from_script();
}
