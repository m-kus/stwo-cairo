use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use stwo_cairo_prover::cairo_air::{
    prove_cairo, verify_cairo, CairoVerificationError, ConfigBuilder,
};
use stwo_cairo_prover::input::vm_import::{adapt_vm_output, VmImportError};
use stwo_cairo_prover::input::ProverInput;
use stwo_cairo_utils::binary_utils::run_binary;
use stwo_prover::core::prover::ProvingError;
use stwo_cairo_serialize::CairoSerialize;
use stwo_prover::core::vcs::poseidon252_merkle::Poseidon252MerkleChannel;
use thiserror::Error;
use tracing::{span, Level};

/// Command line arguments for adapted_stwo.
/// Example command line:
///     ```
///     cargo run -r --bin adapted_stwo -- --pub_json absolute/path/to/pub.json
///     --priv_json absolute/path/to/priv.json --proof_path path/to/proof
///     ```
#[derive(Parser, Debug)]
struct Args {
    #[structopt(long = "pub_json")]
    pub_json: PathBuf,
    #[structopt(long = "priv_json")]
    priv_json: PathBuf,
    /// The output file path for the proof.
    #[structopt(long = "proof_path")]
    proof_path: PathBuf,
    #[structopt(long = "track_relations")]
    track_relations: bool,
    #[structopt(long = "display_components")]
    display_components: bool,
    #[structopt(long = "verify")]
    verify: bool,
}

#[derive(Debug, Error)]
enum Error {
    #[error("Invalid arguments: {0}")]
    Cli(#[from] clap::Error),
    #[error("VM import failed: {0}")]
    VmImport(#[from] VmImportError),
    #[error("Proving failed: {0}")]
    Proving(#[from] ProvingError),
    #[error("Verification failed: {0}")]
    Verification(#[from] CairoVerificationError),
    #[error("Serialization failed: {0}")]
    Serde(#[from] serde_json::error::Error),
    #[error("IO failed: {0}")]
    IO(#[from] std::io::Error),
}

fn main() -> ExitCode {
    run_binary(run)
}

fn run(args: impl Iterator<Item = String>) -> Result<(), Error> {
    let _span = span!(Level::INFO, "run").entered();
    let args = Args::try_parse_from(args)?;

    let vm_output: ProverInput =
        adapt_vm_output(args.pub_json.as_path(), args.priv_json.as_path(), true)?;
    let prover_config = ConfigBuilder::default()
        .track_relations(args.track_relations)
        .display_components(args.display_components)
        .build();

    log::info!(
        "Casm states by opcode:\n {},",
        vm_output.state_transitions.casm_states_by_opcode
    );

    // TODO(Ohad): Propagate hash from CLI args.
    let proof = prove_cairo::<Poseidon252MerkleChannel>(vm_output, prover_config)?;

    let mut output: Vec<starknet_ff::FieldElement> = Vec::new();
    CairoSerialize::serialize(&proof, &mut output);

    let mut result = output
        .into_iter()
        .map(|felt| felt.to_string())
        .reduce(|acc, elt| format!("{acc} {elt}"))
        .unwrap();
    result = format!("[{result}]");

    std::fs::write(args.proof_path, result)?;

    if args.verify {
        verify_cairo::<Poseidon252MerkleChannel>(proof)?;
        log::info!("Proof verified successfully");
    }

    Ok(())
}
