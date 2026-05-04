fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/gateway.proto",
                "proto/users.proto",
                "proto/questions.proto",
                "proto/game_room.proto",
                "proto/score.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
