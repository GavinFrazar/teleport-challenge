fn main() {
    tonic_build::compile_protos("./remotejobs.proto")
        .unwrap_or_else(|err| panic!("Failed to compile protos {:?}", err));
}
