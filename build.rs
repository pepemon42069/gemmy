use std::io::Result;

fn main() -> Result<()> {
    let mut config = prost_build::Config::new();
    let out_dir = "src/protobuf";
    config.out_dir(out_dir);

    config.compile_protos(
        &["resources/protobuf/types.proto"], 
        &["resources/protobuf"])?;
    Ok(())
}

