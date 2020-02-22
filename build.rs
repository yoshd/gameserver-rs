fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/gameserver/game.proto")?;
    tonic_build::configure().build_server(true).compile(
        &[
            "deps/open-match/api/backend.proto",
            "deps/open-match/api/frontend.proto",
            "deps/open-match/api/matchfunction.proto",
            "deps/open-match/api/query.proto",
        ],
        &["deps/open-match", "deps/open-match/third_party"],
    )?;
    tonic_build::compile_protos("src/frontend/game_frontend.proto")?;
    Ok(())
}
