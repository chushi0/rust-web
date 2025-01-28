fn main() {
    tonic_build::compile_protos("../idl/core_rpc.proto").unwrap();
}
