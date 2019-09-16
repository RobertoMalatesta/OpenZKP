use crate::{
    constraint::Constraint,
    pedersen_merkle::{
        inputs::PublicInput,
        periodic_columns::{
            LEFT_X_COEFFICIENTS, LEFT_Y_COEFFICIENTS, RIGHT_X_COEFFICIENTS, RIGHT_Y_COEFFICIENTS,
        },
    },
    polynomial::{DensePolynomial, SparsePolynomial},
};
use elliptic_curve::Affine;
use primefield::FieldElement;
use starkdex::SHIFT_POINT;
use std::{prelude::v1::*, vec};
use u256::U256;

// TODO: Naming
#[allow(clippy::module_name_repetitions)]
pub fn get_pedersen_merkle_constraints(public_input: &PublicInput) -> Vec<Constraint> {
    fn get_left_bit(trace_polynomials: &[DensePolynomial]) -> DensePolynomial {
        &trace_polynomials[0]
            - &FieldElement::from(U256::from(2_u64)) * &trace_polynomials[0].next()
    }
    fn get_right_bit(trace_polynomials: &[DensePolynomial]) -> DensePolynomial {
        &trace_polynomials[4]
            - &FieldElement::from(U256::from(2_u64)) * &trace_polynomials[4].next()
    }

    let path_length = public_input.path_length;
    let trace_length = path_length * 256;
    let root = public_input.root.clone();
    let leaf = public_input.leaf.clone();
    let field_element_bits = 252;

    let g = FieldElement::root(trace_length).unwrap();
    let no_rows = SparsePolynomial::new(&[(FieldElement::ONE, 0)]);
    let first_row = SparsePolynomial::new(&[(-&FieldElement::ONE, 0), (FieldElement::ONE, 1)]);
    let last_row = SparsePolynomial::new(&[(-&g.pow(trace_length - 1), 0), (FieldElement::ONE, 1)]);
    let hash_end_rows = SparsePolynomial::new(&[
        (FieldElement::ONE, path_length),
        (-&g.pow(path_length * (trace_length - 1)), 0),
    ]);
    let field_element_end_rows = SparsePolynomial::new(&[
        (-&g.pow(field_element_bits * path_length), 0),
        (FieldElement::ONE, path_length),
    ]);
    let hash_start_rows =
        SparsePolynomial::new(&[(FieldElement::ONE, path_length), (-&FieldElement::ONE, 0)]);
    let every_row =
        SparsePolynomial::new(&[(FieldElement::ONE, trace_length), (-&FieldElement::ONE, 0)]);

    let (shift_point_x, shift_point_y) = match SHIFT_POINT {
        Affine::Zero => panic!(),
        Affine::Point { x, y } => (x, y),
    };

    let q_x_left_1 = SparsePolynomial::periodic(&LEFT_X_COEFFICIENTS, path_length);
    let q_x_left_2 = SparsePolynomial::periodic(&LEFT_X_COEFFICIENTS, path_length);
    let q_y_left = SparsePolynomial::periodic(&LEFT_Y_COEFFICIENTS, path_length);
    let q_x_right_1 = SparsePolynomial::periodic(&RIGHT_X_COEFFICIENTS, path_length);
    let q_x_right_2 = SparsePolynomial::periodic(&RIGHT_X_COEFFICIENTS, path_length);
    let q_y_right = SparsePolynomial::periodic(&RIGHT_Y_COEFFICIENTS, path_length);

    vec![
        Constraint {
            base:        Box::new(|tp| tp[0].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[1].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[2].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[3].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[4].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[5].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[6].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| tp[7].clone()),
            numerator:   no_rows.clone(),
            denominator: no_rows.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                (SparsePolynomial::new(&[(leaf.clone(), 0)]) - &tp[0])
                    * (SparsePolynomial::new(&[(leaf.clone(), 0)]) - &tp[4])
            }),
            numerator:   no_rows.clone(),
            denominator: first_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| SparsePolynomial::new(&[(root.clone(), 0)]) - &tp[6]),
            numerator:   no_rows.clone(),
            denominator: last_row.clone(),
        },
        Constraint {
            base:        Box::new(|tp| (&tp[6] - tp[0].next()) * (&tp[6] - tp[4].next())),
            numerator:   last_row.clone(),
            denominator: hash_end_rows.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                &tp[6] - SparsePolynomial::new(&[(shift_point_x.clone(), 0)])
            }),
            numerator:   no_rows.clone(),
            denominator: hash_start_rows.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                &tp[7] - SparsePolynomial::new(&[(shift_point_y.clone(), 0)])
            }),
            numerator:   no_rows.clone(),
            denominator: hash_start_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| {
                let left_bit = get_left_bit(tp);
                &left_bit * (&left_bit - SparsePolynomial::new(&[(FieldElement::ONE, 0)]))
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let left_bit = get_left_bit(tp);
                left_bit * (&tp[7] - q_y_left.clone())
                    - tp[1].next() * (&tp[6] - q_x_left_1.clone())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let left_bit = get_left_bit(tp);
                tp[1].next().square() - left_bit * (&tp[6] + q_x_left_2.clone() + tp[2].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let left_bit = get_left_bit(tp);
                &left_bit * (tp[7].clone() + tp[3].next())
                    - tp[1].next() * (tp[6].clone() - tp[2].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let left_bit = get_left_bit(tp);
                (SparsePolynomial::new(&[(FieldElement::ONE, 0)]) - &left_bit)
                    * (tp[6].clone() - tp[2].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let left_bit = get_left_bit(tp);
                (SparsePolynomial::new(&[(FieldElement::ONE, 0)]) - &left_bit)
                    * (tp[7].clone() - tp[3].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| tp[0].clone()),
            numerator:   no_rows.clone(),
            denominator: field_element_end_rows.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| tp[0].clone()),
            numerator:   no_rows.clone(),
            denominator: hash_end_rows.clone(),
        },
        Constraint {
            base:        Box::new(|tp| {
                let right_bit = get_right_bit(tp);
                right_bit.clone() * (&right_bit - SparsePolynomial::new(&[(FieldElement::ONE, 0)]))
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let right_bit = get_right_bit(tp);
                right_bit * (&tp[3].next() - q_y_right.clone())
                    - tp[5].next() * (&tp[2].next() - q_x_right_1.clone())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let right_bit = get_right_bit(tp);
                tp[5].next().square()
                    - right_bit * (&tp[2].next() + q_x_right_2.clone() + tp[6].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let right_bit = get_right_bit(tp);
                &right_bit * (tp[3].next() + tp[7].next())
                    - tp[5].next() * (tp[2].next() - tp[6].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let right_bit = get_right_bit(tp);
                (SparsePolynomial::new(&[(FieldElement::ONE, 0)]) - &right_bit)
                    * (tp[2].next() - tp[6].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| {
                let right_bit = get_right_bit(tp);
                (SparsePolynomial::new(&[(FieldElement::ONE, 0)]) - &right_bit)
                    * (tp[3].next() - tp[7].next())
            }),
            numerator:   hash_end_rows.clone(),
            denominator: every_row.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| tp[4].clone()),
            numerator:   no_rows.clone(),
            denominator: field_element_end_rows.clone(),
        },
        Constraint {
            base:        Box::new(move |tp| tp[4].clone()),
            numerator:   no_rows.clone(),
            denominator: hash_end_rows.clone(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pedersen_merkle::{
            inputs::{short_private_input, SHORT_PUBLIC_INPUT},
            trace_table::get_trace_table,
        },
        proof_params::ProofParams,
        proofs::stark_proof,
    };

    // TODO: Implement verifier and re-enable
    #[ignore]
    #[test]
    fn short_pedersen_merkle() {
        let public_input = SHORT_PUBLIC_INPUT;
        let private_input = short_private_input();
        let trace_table = get_trace_table(&public_input, &private_input);

        let constraints = &get_pedersen_merkle_constraints(&public_input);

        let proof = stark_proof(&trace_table, &constraints, &public_input, &ProofParams {
            blowup:                   16,
            pow_bits:                 0,
            queries:                  13,
            fri_layout:               vec![3, 2],
            constraints_degree_bound: 2,
        });

        assert_eq!(
            hex::encode(proof.proof[0..32].to_vec()),
            "e2c4e35c37e33aa3b439592f2f3c5c82f464f026000000000000000000000000"
        );
        assert_eq!(
            hex::encode(proof.proof[32..64].to_vec()),
            "c5df989253ac4c3eff4fdb4130f832db1d2a9826000000000000000000000000"
        );

        // FRI commitments
        assert_eq!(
            hex::encode(proof.proof[640..672].to_vec()),
            "744f04f8bcd9c5aafb8907586428fbe9dd81b976000000000000000000000000"
        );
        assert_eq!(
            hex::encode(proof.proof[672..704].to_vec()),
            "ce329839a5eccb8009ffebf029312989e68f1cde000000000000000000000000"
        );
    }
}