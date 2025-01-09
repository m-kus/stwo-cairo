use stwo_cairo_air::{CairoProof, verify_cairo};

//#[executable]
fn main(arguments: Array<felt252>) -> Array<felt252> {
    let mut args = arguments.span();
    let proof: CairoProof = Serde::deserialize(ref args).expect('Failed to deserialize');

    if let Result::Err(err) = verify_cairo(proof) {
        panic!("Verification failed: {:?}", err);
    }

    array![]
}
