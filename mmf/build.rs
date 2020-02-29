fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(true).compile(
        &[
            "../deps/open-match/api/matchfunction.proto",
            "../deps/open-match/api/query.proto",
        ],
        &["../deps/open-match", "../deps/open-match/third_party"],
    )?;
    Ok(())
}
