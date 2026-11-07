//! SSL2 entry points, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/ssl2.cpp`.
//!
//! The C++ implementation is a stub table: every function logs and returns `ORBIS_OK`.

use crate::gpu::core::libraries::error_codes::ORBIS_OK;

/// The export names registered by `RegisterLib`.
pub const EXPORTS: [&str; 44] = [
    "CA_MGMT_extractKeyBlobEx",
    "CA_MGMT_extractPublicKeyInfo",
    "CA_MGMT_freeKeyBlob",
    "CRYPTO_initAsymmetricKey",
    "CRYPTO_uninitAsymmetricKey",
    "RSA_verifySignature",
    "sceSslCheckRecvPending",
    "sceSslClose",
    "sceSslConnect",
    "sceSslCreateConnection",
    "sceSslCreateSslConnection",
    "sceSslDeleteConnection",
    "sceSslDeleteSslConnection",
    "sceSslDisableOption",
    "sceSslDisableOptionInternal",
    "sceSslDisableOptionInternalInsecure",
    "sceSslDisableVerifyOption",
    "sceSslEnableOption",
    "sceSslEnableOptionInternal",
    "sceSslEnableVerifyOption",
    "sceSslFreeCaCerts",
    "sceSslFreeCaList",
    "sceSslFreeSslCertName",
    "sceSslGetAlpn",
    "sceSslGetCaCerts",
    "sceSslGetCaList",
    "sceSslGetIssuerName",
    "sceSslGetMemoryPoolStats",
    "sceSslGetNameEntryCount",
    "sceSslGetNameEntryInfo",
    "sceSslGetNanoSSLModuleId",
    "sceSslGetNotAfter",
    "sceSslGetNotBefore",
    "sceSslGetSerialNumber",
    "sceSslGetSslError",
    "sceSslGetSubjectName",
    "sceSslInit",
    "sceSslLoadCert",
    "sceSslLoadRootCACert",
    "sceSslRead",
    "sceSslRecv",
    "sceSslReuseConnection",
    "sceSslSend",
    "sceSslSetAlpn",
];

/// `Ssl2`; every entry point is the same stub.
#[derive(Debug, Default)]
pub struct Ssl2;

impl Ssl2 {
    pub fn call(&mut self, _name: &str) -> i32 {
        ORBIS_OK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssl2_stubs_return_ok() {
        let mut ssl = Ssl2;
        assert_eq!(ssl.call("sceSslInit"), ORBIS_OK);
        assert_eq!(ssl.call("unknown"), ORBIS_OK);
        assert_eq!(EXPORTS.len(), 44);
    }
}
