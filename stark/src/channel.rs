// TODO: Naming?
#![allow(clippy::module_name_repetitions)]
use crate::{hash::Hash, MerkleProof};
use macros_decl::{hex, u256h};
use primefield::FieldElement;
use std::prelude::v1::*;
use tiny_keccak::Keccak;
use u256::U256;

#[cfg(feature = "std")]
use rayon::prelude::*;

pub trait RandomGenerator<T> {
    fn get_random(&mut self) -> T;
}

pub trait Writable<T> {
    fn write(&mut self, data: T);
}

pub trait Replayable<T> {
    fn replay(&mut self) -> T;

    fn replay_many(&mut self, count: usize) -> Vec<T> {
        (0..count).map(|_| self.replay()).collect()
    }
}

#[derive(PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct PublicCoin {
    pub digest: [u8; 32],
    counter:    u64,
}

#[derive(PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ProverChannel {
    pub coin:  PublicCoin,
    pub proof: Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct VerifierChannel {
    pub coin:    PublicCoin,
    pub proof:   Vec<u8>,
    proof_index: usize,
}

impl PublicCoin {
    pub fn new() -> Self {
        Self {
            digest:  [0; 32],
            counter: 0,
        }
    }

    pub fn seed(&mut self, seed: &[u8]) {
        let mut keccak = Keccak::new_keccak256();
        keccak.update(seed);
        keccak.finalize(&mut self.digest);
        self.counter = 0;
    }

    pub fn pow_find_nonce(&self, pow_bits: u8) -> u64 {
        let seed = self.pow_seed(pow_bits);

        // We assume a nonce exists and will be found in reasonable time.
        #[allow(clippy::maybe_infinite_iter)]
        (0_u64..)
            .find(|&nonce| Self::pow_verify_with_seed(nonce, pow_bits, &seed))
            .expect("No valid nonce found")
    }

    // TODO - Make tests compatible with the proof of work values from this function
    #[cfg(feature = "std")]
    pub fn pow_find_nonce_threaded(&self, pow_bits: u8) -> u64 {
        let seed = self.pow_seed(pow_bits);
        // NOTE: Rayon does not support open ended ranges, so we need to use a closed
        // one.
        (0..u64::max_value())
            .into_par_iter()
            .find_any(|&nonce| Self::pow_verify_with_seed(nonce, pow_bits, &seed))
            .expect("No valid nonce found")
    }

    pub fn pow_seed(&self, pow_bits: u8) -> [u8; 32] {
        let mut seed = [0_u8; 32];
        let mut keccak = Keccak::new_keccak256();
        keccak.update(&hex!("0123456789abcded"));
        keccak.update(&self.digest);
        keccak.update(&[pow_bits]);
        keccak.finalize(&mut seed);
        seed
    }

    pub fn pow_verify(&self, nonce: u64, pow_bits: u8) -> bool {
        let seed = self.pow_seed(pow_bits);
        Self::pow_verify_with_seed(nonce, pow_bits, &seed)
    }

    fn pow_verify_with_seed(nonce: u64, pow_bits: u8, seed: &[u8; 32]) -> bool {
        // OPT: Inline Keccak256 and work directly on buffer using 'keccakf'
        let mut keccak = Keccak::new_keccak256();
        let mut digest = [0_u8; 32];
        keccak.update(seed);
        keccak.update(&(nonce.to_be_bytes()));
        keccak.finalize(&mut digest);
        // OPT: Check performance impact of conversion
        let work = U256::from_bytes_be(&digest).leading_zeros();
        work >= pow_bits as usize
    }
}

impl From<Vec<u8>> for ProverChannel {
    fn from(proof_data: Vec<u8>) -> Self {
        Self {
            coin:  PublicCoin::new(),
            proof: proof_data,
        }
    }
}

impl ProverChannel {
    pub fn new() -> Self {
        Self {
            coin:  PublicCoin::new(),
            proof: Vec::new(),
        }
    }

    pub fn initialize(&mut self, seed: &[u8]) {
        self.coin.seed(seed);
    }

    pub fn pow_verify(&self, nonce: u64, pow_bits: u8) -> bool {
        self.coin.pow_verify(nonce, pow_bits)
    }

    pub fn pow_find_nonce(&self, pow_bits: u8) -> u64 {
        self.coin.pow_find_nonce(pow_bits)
    }

    #[cfg(feature = "std")]
    pub fn pow_find_nonce_threaded(&self, pow_bits: u8) -> u64 {
        self.coin.pow_find_nonce_threaded(pow_bits)
    }
}

impl VerifierChannel {
    pub fn new(proof: Vec<u8>) -> Self {
        Self {
            coin: PublicCoin::new(),
            proof,
            proof_index: 0,
        }
    }

    pub fn initialize(&mut self, seed: &[u8]) {
        self.coin.seed(seed);
    }

    pub fn pow_verify(&self, nonce: u64, pow_bits: u8) -> bool {
        self.coin.pow_verify(nonce, pow_bits)
    }

    pub fn pow_find_nonce(&self, pow_bits: u8) -> u64 {
        self.coin.pow_find_nonce(pow_bits)
    }

    #[cfg(feature = "std")]
    pub fn pow_find_nonce_threaded(&self, pow_bits: u8) -> u64 {
        self.coin.pow_find_nonce_threaded(pow_bits)
    }

    pub fn read_without_replay(&self, length: usize) -> &[u8] {
        &self.proof[self.proof_index..(self.proof_index + length)]
    }

    pub fn at_end(self) -> bool {
        self.proof_index == self.proof.len()
    }
}

impl RandomGenerator<FieldElement> for PublicCoin {
    fn get_random(&mut self) -> FieldElement {
        const MASK: U256 =
            u256h!("0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        loop {
            let number: U256 = self.get_random();
            let seed = number & MASK;
            if seed < FieldElement::MODULUS {
                // TODO: Avoid accessing FieldElement members directly
                break FieldElement::from_montgomery(seed);
            }
        }
    }
}

impl RandomGenerator<U256> for PublicCoin {
    fn get_random(&mut self) -> U256 {
        U256::from_bytes_be(&self.get_random())
    }
}

impl RandomGenerator<[u8; 32]> for PublicCoin {
    fn get_random(&mut self) -> [u8; 32] {
        let mut result = [0; 32];
        let mut keccak = Keccak::new_keccak256();
        keccak.update(&self.digest);
        keccak.update(&[0_u8; 24]);
        keccak.update(&self.counter.to_be_bytes());
        keccak.finalize(&mut result);
        self.counter += 1;
        result
    }
}

impl<T> RandomGenerator<T> for ProverChannel
where
    PublicCoin: RandomGenerator<T>,
{
    fn get_random(&mut self) -> T {
        self.coin.get_random()
    }
}

impl<T> RandomGenerator<T> for VerifierChannel
where
    PublicCoin: RandomGenerator<T>,
{
    fn get_random(&mut self) -> T {
        self.coin.get_random()
    }
}

impl Writable<&[u8]> for PublicCoin {
    fn write(&mut self, data: &[u8]) {
        let mut result: [u8; 32] = [0; 32];
        let mut keccak = Keccak::new_keccak256();
        keccak.update(&self.digest);
        keccak.update(data);
        keccak.finalize(&mut result);
        self.digest = result;
        self.counter = 0;
    }
}

// Note - that this default implementation allows writing a sequence of &[u8] to
// the proof with the same encoding for the writing and the non writing. However
// by writing directly to the coin, other writes for the channel could separate
// encoding from random perturbation.
impl Writable<&[u8]> for ProverChannel {
    fn write(&mut self, data: &[u8]) {
        self.proof.extend_from_slice(data);
        self.coin.write(data);
    }
}

impl Writable<&Hash> for ProverChannel {
    fn write(&mut self, data: &Hash) {
        self.write(data.as_bytes());
    }
}

impl Writable<u64> for ProverChannel {
    fn write(&mut self, data: u64) {
        self.write(&data.to_be_bytes()[..]);
    }
}

// OPT - Remove allocation of vectors
impl Writable<&[FieldElement]> for ProverChannel {
    fn write(&mut self, data: &[FieldElement]) {
        let mut container = Vec::with_capacity(32 * data.len());
        for element in data {
            for byte in &element.as_montgomery().to_bytes_be() {
                container.push(byte.clone());
            }
        }
        self.write(container.as_slice());
    }
}

impl Writable<&FieldElement> for ProverChannel {
    fn write(&mut self, data: &FieldElement) {
        // TODO: Avoid accessing FieldElement members directly
        self.write(&data.as_montgomery().to_bytes_be()[..]);
    }
}

// Note -- This method of writing is distinct from the field element, and is
// used in the decommitment when groups are decommited from the rows
impl Writable<Vec<U256>> for ProverChannel {
    fn write(&mut self, data: Vec<U256>) {
        for element in data {
            self.write(element)
        }
    }
}

impl Writable<U256> for ProverChannel {
    fn write(&mut self, data: U256) {
        self.write(&data.to_bytes_be()[..]);
    }
}

impl Writable<&MerkleProof> for ProverChannel {
    fn write(&mut self, proof: &MerkleProof) {
        for decommitment in proof.decommitments() {
            self.write(decommitment)
        }
    }
}

impl Replayable<Hash> for VerifierChannel {
    fn replay(&mut self) -> Hash {
        let hash: [u8; 32] = self.replay();
        Hash::new(hash)
    }
}

impl Replayable<[u8; 32]> for VerifierChannel {
    fn replay(&mut self) -> [u8; 32] {
        let mut holder = [0_u8; 32];
        let from = self.proof_index;
        let to = from + 32;
        self.proof_index = to;
        // OPT: Use arrayref crate or similar to avoid copy
        holder.copy_from_slice(&self.proof[from..to]);
        self.coin.write(&holder[..]);
        holder
    }
}

impl Replayable<U256> for VerifierChannel {
    fn replay(&mut self) -> U256 {
        U256::from_bytes_be(&Replayable::replay(self))
    }
}

impl Replayable<FieldElement> for VerifierChannel {
    fn replay(&mut self) -> FieldElement {
        FieldElement::from_montgomery(Replayable::replay(self))
    }

    fn replay_many(&mut self, len: usize) -> Vec<FieldElement> {
        let start_index = self.proof_index;
        let mut ret = Vec::with_capacity(len);
        for _ in 0..len {
            let mut holder = [0_u8; 32];
            let from = self.proof_index;
            let to = from + 32;
            self.proof_index = to;
            holder.copy_from_slice(&self.proof[from..to]);
            ret.push(FieldElement::from_montgomery(U256::from_bytes_be(&holder)));
        }
        self.coin.write(&self.proof[start_index..self.proof_index]);
        ret
    }
}

impl Replayable<u64> for VerifierChannel {
    fn replay(&mut self) -> u64 {
        let mut holder = [0_u8; 8];
        let from = self.proof_index;
        let to = from + 8;
        self.proof_index = to;
        // OPT: Use arrayref crate or similar to avoid copy
        holder.copy_from_slice(&self.proof[from..to]);
        self.coin.write(&holder[..]);
        u64::from_be_bytes(holder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macros_decl::u256h;

    #[test]
    fn proof_of_work_test() {
        let mut rand_source = ProverChannel::new();
        rand_source.initialize(hex!("0123456789abcded").to_vec().as_slice());
        let mut ver_rand_source = VerifierChannel::new(rand_source.proof.clone());
        ver_rand_source.initialize(&hex!("0123456789abcded"));
        let work = rand_source.pow_find_nonce(12);
        let ver_work = ver_rand_source.pow_find_nonce(12);
        assert_eq!(ver_work, work);
        assert!(&rand_source.pow_verify(work, 12));
    }

    #[test]
    fn threaded_proof_of_work_test() {
        let mut rand_source = ProverChannel::new();
        rand_source.initialize(&hex!("0123456789abcded"));
        let work = rand_source.pow_find_nonce_threaded(12);
        assert!(&rand_source.pow_verify(work, 12));
    }

    #[test]
    fn ver_threaded_proof_of_work_test() {
        let mut rand_source = VerifierChannel::new(hex!("0123456789abcded").to_vec());
        rand_source.initialize(&hex!("0123456789abcded"));
        let work = rand_source.pow_find_nonce_threaded(12);
        assert!(&rand_source.pow_verify(work, 12));
    }

    // Note - This test depends on the specific ordering of the subtests because of
    // the nature of the channel
    #[test]
    fn test_channel_get_random() {
        let mut source = ProverChannel::new();
        source.initialize(hex!("0123456789abcded").to_vec().as_slice());
        let rand_bytes: [u8; 32] = source.get_random();
        assert_eq!(
            rand_bytes,
            hex!("7d84f75ca3e9328b92123c1790834ee0084e02c09b379c6f95c5d2ae8739b9c8")
        );
        let rand_int: U256 = source.get_random();
        assert_eq!(
            rand_int,
            u256h!("4ed5f0fd8cffa8dec69beebab09ee881e7369d6d084b90208a079eedc67d2d45")
        );
        let rand_element: FieldElement = source.get_random();
        assert_eq!(
            rand_element,
            FieldElement::from_montgomery(u256h!(
                "0389a47fe0e1e5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
            ))
        );
    }

    // Note - This test depends on the specific ordering of the subtests because of
    // the nature of the channel
    #[test]
    fn test_channel_write() {
        let mut source = ProverChannel::new();
        source.initialize(&hex!("0123456789abcded"));
        let rand_bytes: [u8; 32] = source.get_random();
        source.write(&rand_bytes[..]);
        assert_eq!(
            source.coin.digest,
            hex!("3174a00d031bc8deff799e24a78ee347b303295a6cb61986a49873d9b6f13a0d")
        );
        source.write(11_028_357_238_u64);
        assert_eq!(
            source.coin.digest,
            hex!("21571e2a323daa1e6f2adda87ce912608e1325492d868e8fe41626633d6acb93")
        );
        source.write(&FieldElement::from_montgomery(u256h!(
            "0389a47fe0e1e5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
        )));
        assert_eq!(
            source.coin.digest,
            hex!("34a12938f047c34da72b5949434950fa2b24220270fd26e6f64b6eb5e86c6626")
        );
        source.write(
            vec![
                FieldElement::from_montgomery(u256h!(
                    "0389a47fe0e1e5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
                )),
                FieldElement::from_montgomery(u256h!(
                    "129ab47fe0e1a5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
                )),
            ]
            .as_slice(),
        );
        assert_eq!(
            source.coin.digest,
            hex!("a748ff89e2c4322afb061ef3321e207b3fe32c35f181de0809300995dd9b92fd")
        );
    }

    #[test]
    fn verifier_channel_test() {
        let mut source = ProverChannel::new();
        source.initialize(&hex!("0123456789abcded"));
        let rand_bytes: [u8; 32] = source.get_random();
        source.write(&rand_bytes[..]);
        source.write(11_028_357_238_u64);
        let written_field_element = FieldElement::from_montgomery(u256h!(
            "0389a47fe0e1e5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
        ));
        source.write(&written_field_element);
        let written_field_element_vec = vec![
            FieldElement::from_montgomery(u256h!(
                "0389a47fe0e1e5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
            )),
            FieldElement::from_montgomery(u256h!(
                "129ab47fe0e1a5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"
            )),
        ];
        source.write(written_field_element_vec.as_slice());

        let written_big_int_vec = vec![
            u256h!("0389a47fe0e1e5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"),
            u256h!("129ab47fe0e1a5f9c05d8dcb27b069b67b1c7ec61a5c0a3f54d81aea83d2c8f0"),
        ];
        source.write(written_big_int_vec.clone());

        let mut verifier = VerifierChannel::new(source.proof.clone());
        verifier.initialize(&hex!("0123456789abcded"));
        let bytes_test: [u8; 32] = verifier.replay();
        assert_eq!(bytes_test, rand_bytes);
        assert_eq!(
            verifier.coin.digest,
            hex!("3174a00d031bc8deff799e24a78ee347b303295a6cb61986a49873d9b6f13a0d")
        );
        let integer_test: u64 = verifier.replay();
        assert_eq!(integer_test, 11_028_357_238_u64);
        assert_eq!(
            verifier.coin.digest,
            hex!("21571e2a323daa1e6f2adda87ce912608e1325492d868e8fe41626633d6acb93")
        );
        let field_element_test: FieldElement = verifier.replay();
        assert_eq!(field_element_test, written_field_element);
        assert_eq!(
            verifier.coin.digest,
            hex!("34a12938f047c34da72b5949434950fa2b24220270fd26e6f64b6eb5e86c6626")
        );
        let field_element_vec_test: Vec<FieldElement> = verifier.replay_many(2);
        assert_eq!(field_element_vec_test, written_field_element_vec);
        assert_eq!(
            verifier.coin.digest,
            hex!("a748ff89e2c4322afb061ef3321e207b3fe32c35f181de0809300995dd9b92fd")
        );
        let bit_int_vec_test: Vec<U256> = verifier.replay_many(2);
        assert_eq!(bit_int_vec_test, written_big_int_vec);
        assert_eq!(verifier.coin.digest, source.coin.digest);
    }
}
