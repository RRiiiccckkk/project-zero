use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use rand::rngs::OsRng;

pub struct NodeIdentity {
    keypair: SigningKey,
}

impl NodeIdentity {
    pub fn new() -> Self {
        let mut csprng = OsRng; // 修复：必须是 mut
        let keypair = SigningKey::generate(&mut csprng);
        Self { keypair }
    }

    pub fn get_public_key(&self) -> VerifyingKey {
        self.keypair.verifying_key()
    }

    pub fn sign(&self, message: &[u8]) -> ed25519_dalek::Signature {
        self.keypair.sign(message)
    }
}