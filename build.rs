use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .out_dir("src/protobuf")
        .compile_protos(&["resources/protobuf/models.proto", "resources/protobuf/services.proto"],
        &["resources/protobuf"])?;

    Ok(())
}

