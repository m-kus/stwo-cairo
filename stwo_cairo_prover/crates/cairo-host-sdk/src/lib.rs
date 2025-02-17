use anyhow::Context;
use cairo_lang_executable::executable::{EntryPointKind, Executable};
use cairo_lang_runner::{build_hints_dict, Arg, CairoHintProcessor};
use cairo_vm::cairo_run::{cairo_run_program, CairoRunConfig};
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::Felt252;
use stwo_cairo_prover::cairo_air::CairoProof;
use stwo_cairo_prover::adapter::plain::adapt_finished_runner;
use stwo_cairo_prover::adapter::ProverInput;
use stwo_prover::core::vcs::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo_cairo_serialize::CairoSerialize;
use stwo_cairo_prover::cairo_air::{prove_cairo, verify_cairo};
use stwo_prover::core::vcs::poseidon252_merkle::{Poseidon252MerkleChannel, Poseidon252MerkleHasher};

pub fn load_executable(bytes: &[u8]) -> anyhow::Result<Executable> {
    let executable: Executable = serde_json::from_slice(bytes)
        .with_context(|| "failed to deserialize executable program")?;

    Ok(executable)
}

pub fn load_program(executable: &Executable) -> anyhow::Result<Program> {
    let data = executable
        .program
        .bytecode
        .iter()
        .map(Felt252::from)
        .map(MaybeRelocatable::from)
        .collect();

    let entrypoint = executable
        .entrypoints
        .iter()
        .find(|e| matches!(e.kind, EntryPointKind::Standalone))
        .with_context(|| "no `Standalone` entrypoint found")?;

    let (hints, _) = build_hints_dict(&executable.program.hints);

    let program = Program::new_for_proof(
        entrypoint.builtins.clone(),
        data,
        entrypoint.offset,
        entrypoint.offset + 4,
        hints,
        Default::default(),
        Default::default(),
        vec![],
        None,
    )
    .with_context(|| "failed setting up program")?;

    Ok(program)
}

pub fn build_program_input<T: CairoSerialize>(executable: &Executable, arg: T) -> anyhow::Result<CairoHintProcessor<'static>> {
    let (_, string_to_hint) = build_hints_dict(&executable.program.hints);

    let mut serialized: Vec<starknet_ff::FieldElement> = Vec::new();
    CairoSerialize::serialize(&arg, &mut serialized);

    let user_args = serialized
        .into_iter()
        .map(|arg| Arg::Value(Felt252::from_bytes_be(&arg.to_bytes_be())))
        .collect();

    let hint_processor = CairoHintProcessor {
        runner: None,
        user_args: vec![vec![Arg::Array(user_args)]],
        string_to_hint,
        starknet_state: Default::default(),
        run_resources: Default::default(),
        syscalls_used_resources: Default::default(),
        no_temporary_segments: false,
        markers: Default::default(),
    };

    Ok(hint_processor)
}

pub fn build_program_config(proof_mode: bool) -> anyhow::Result<CairoRunConfig<'static>> {
    let cairo_run_config = CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        proof_mode,
        secure_run: None,
        relocate_mem: true,
        trace_enabled: true,
        ..Default::default()
    };  

    Ok(cairo_run_config)
}

pub fn execute(program: Program, hint_processor: &mut CairoHintProcessor, cairo_run_config: CairoRunConfig) -> anyhow::Result<ProverInput> {
    let runner = cairo_run_program(&program, &cairo_run_config, hint_processor)
        .with_context(|| "Cairo program run failed")?;

    let prover_input = adapt_finished_runner(runner)?;

    Ok(prover_input)
}

pub enum Proof {
    ProofBlake2s(CairoProof<Blake2sMerkleHasher>),
    ProofPoseidon252(CairoProof<Poseidon252MerkleHasher>),
}

pub fn prove(prover_input: ProverInput) -> anyhow::Result<Proof> {
    let proof = prove_cairo::<Blake2sMerkleChannel>(prover_input, Default::default())
        .context("failed to generate proof")?;
    Ok(Proof::ProofBlake2s(proof))
}

pub fn verify(proof: Proof) -> anyhow::Result<()> {
    match proof {
        Proof::ProofBlake2s(proof) => verify_cairo::<Blake2sMerkleChannel>(proof)
            .context("failed to verify proof"),
        Proof::ProofPoseidon252(proof) => verify_cairo::<Poseidon252MerkleChannel>(proof)
            .context("failed to verify proof"),
    }
}
