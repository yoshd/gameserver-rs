fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/game.proto")?;
    tonic_build::compile_protos("../proto/game_frontend.proto")?;
    Ok(())
}
