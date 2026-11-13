//! Handtekeningen op een besteding (stap 5, besluit sectie 0 punt 3).
//!
//! De keten verifieert alleen; tekenen gebeurt in de portemonnee. Het schema
//! zit achter een trait, zodat Falcon later dezelfde plek kan innemen zonder
//! dat de bestedingscode verandert.
//!
//! Eerste invulling: Ed25519 via `ed25519-dalek` (exact gepind in
//! `Cargo.toml`), uitsluitend met `verify_strict`. Die weigert publieke
//! sleutels en R-waarden van kleine orde en een niet-canonieke S; gewone
//! `verify` laat de eerste twee door.

/// Waarom een handtekening geweigerd wordt.
#[derive(Debug, PartialEq, Eq)]
pub enum SigFout {
    /// De publieke sleutel heeft niet precies `PK_BYTES` bytes.
    PkLengte { verwacht: usize, kreeg: usize },
    /// De handtekening heeft niet precies `SIG_BYTES` bytes.
    SigLengte { verwacht: usize, kreeg: usize },
    /// De bytes vormen geen geldige publieke sleutel.
    OngeldigeSleutel,
    /// De handtekening klopt niet voor deze sleutel en dit bericht.
    Ongeldig,
}

/// Een handtekeningschema zoals de keten het gebruikt.
pub trait Handtekening {
    /// Naam van het schema; gaat in `note::owner_hash`, zodat dezelfde
    /// sleutelbytes onder twee schema's nooit dezelfde eigenaar zijn.
    const SCHEMA: &'static [u8];
    /// Byte in het bestedingsrecord.
    const SCHEMA_ID: u8;
    const PK_BYTES: usize;
    const SIG_BYTES: usize;

    /// Leest een publieke sleutel in zonder iets te verifiëren: exacte lengte
    /// en een geldige codering. De besteding roept dit vóór alles wat de
    /// sleutel gebruikt (stap 7, controle 2).
    fn lees_sleutel(pk: &[u8]) -> Result<(), SigFout>;

    /// Verifieert `sig` over `bericht` onder `pk`. Lengtes moeten exact
    /// kloppen; er wordt niets afgekapt of aangevuld.
    fn verifieer(pk: &[u8], bericht: &[u8], sig: &[u8]) -> Result<(), SigFout>;
}

/// Ed25519 (RFC 8032), puur: geen prehash, geen context.
#[derive(Debug, Clone, Copy)]
pub struct Ed25519;

impl Handtekening for Ed25519 {
    const SCHEMA: &'static [u8] = b"ed25519";
    const SCHEMA_ID: u8 = 0x01;
    const PK_BYTES: usize = 32;
    const SIG_BYTES: usize = 64;

    fn lees_sleutel(pk: &[u8]) -> Result<(), SigFout> {
        sleutel(pk).map(|_| ())
    }

    fn verifieer(pk: &[u8], bericht: &[u8], sig: &[u8]) -> Result<(), SigFout> {
        let sleutel = sleutel(pk)?;
        let sig: &[u8; 64] = sig
            .try_into()
            .map_err(|_| SigFout::SigLengte { verwacht: Self::SIG_BYTES, kreeg: sig.len() })?;
        let handtekening = ed25519_dalek::Signature::from_bytes(sig);
        sleutel.verify_strict(bericht, &handtekening).map_err(|_| SigFout::Ongeldig)
    }
}

fn sleutel(pk: &[u8]) -> Result<ed25519_dalek::VerifyingKey, SigFout> {
    let pk: &[u8; 32] = pk
        .try_into()
        .map_err(|_| SigFout::PkLengte { verwacht: Ed25519::PK_BYTES, kreeg: pk.len() })?;
    ed25519_dalek::VerifyingKey::from_bytes(pk).map_err(|_| SigFout::OngeldigeSleutel)
}
