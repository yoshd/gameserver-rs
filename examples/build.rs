fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../gameserver/game.proto")?;
    tonic_build::compile_protos("../frontend/game_frontend.proto")?;
    Ok(())
}
