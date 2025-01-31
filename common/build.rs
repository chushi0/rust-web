fn main() {
    println!("cargo:rerun-if-changed=../idl/core_rpc.proto");
    println!("cargo:rerun-if-changed=../idl/mc_service.proto");

    tonic_build::compile_protos("../idl/core_rpc.proto").unwrap();
    tonic_build::compile_protos("../idl/mc_service.proto").unwrap();
}
