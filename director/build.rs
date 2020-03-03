fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(true).compile(
        &["../deps/open-match/api/backend.proto"],
        &["../deps/open-match", "../deps/open-match/third_party"],
    )?;
    Ok(())
}
