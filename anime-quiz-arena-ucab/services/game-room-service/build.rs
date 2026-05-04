fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(true).compile_protos(
        &["../../proto/game_room.proto", "../../proto/score.proto"],
        &["../../proto"],
    )?;
    Ok(())
}
