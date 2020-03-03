fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(true).compile(
        &["../deps/open-match/api/frontend.proto"],
        &["../deps/open-match", "../deps/open-match/third_party"],
    )?;
    tonic_build::compile_protos("../proto/game_frontend.proto")?;
    Ok(())
}
