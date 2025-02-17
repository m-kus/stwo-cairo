use cairo_host_sdk::{
    build_program_config, build_program_input, execute, load_executable, load_program, prove,
    verify,
};

// Include the built Cairo executable
const CAIRO_EXECUTABLE: &[u8] = include_bytes!(std::env!("CAIRO_PROGRAM_PATH"));

fn main() {
    let executable = load_executable(CAIRO_EXECUTABLE).expect("Failed to load executable");
    let program = load_program(&executable).expect("Failed to load program");

    let args: Vec<u32> = vec![];

    let mut hint_processor =
        build_program_input(&executable, args).expect("Failed to build hint processor");
    let cairo_run_config = build_program_config(true).expect("Failed to build cairo run config");

    let prover_input =
        execute(program, &mut hint_processor, cairo_run_config).expect("Failed to execute");
    let proof = prove(prover_input).expect("Failed to prove");

    verify(proof).expect("Failed to verify");
}
