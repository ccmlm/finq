use {
    ruc::*,
    serde::Deserialize,
    zei::{
        serialization::ZeiFromToBytes,
        xfr::{
            sig::{XfrKeyPair, XfrPublicKey},
            structs::{
                AssetRecord, AssetType, BlindAssetRecord, XfrAmount, XfrAssetType, XfrBody,
                ASSET_TYPE_LENGTH,
            },
        },
    },
};

const PK_LEN: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;
const BH_PK_BYTES: [u8; PK_LEN] = [0; PK_LEN];
const BH_PK_STAKING_BYTES: [u8; PK_LEN] = [1; PK_LEN];

const ASSET_TYPE_FRA: AssetType = AssetType([0; ASSET_TYPE_LENGTH]);
const FRA_DECIMALS: u8 = 6;

lazy_static::lazy_static! {
    static ref BH_PK: XfrPublicKey = pnk!(XfrPublicKey::zei_from_bytes(&BH_PK_BYTES[..]));
    static ref BH_PK_STAKING: XfrPublicKey = pnk!(XfrPublicKey::zei_from_bytes(&BH_PK_STAKING_BYTES[..]));
}

const ADDR_LIST: [&str; 9] = [
    "fra1s9c6p0656as48w8su2gxntc3zfuud7m66847j6yh7n8wezazws3s68p0m9",
    "fra1zjfttcnvyv9ypy2d4rcg7t4tw8n88fsdzpggr0y2h827kx5qxmjshwrlx7",
    "fra18rfyc9vfyacssmr5x7ku7udyd5j5vmfkfejkycr06e4as8x7n3dqwlrjrc",
    "fra1kvf8z5f5m8wmp2wfkscds45xv3yp384eszu2mpre836x09mq5cqsknltvj",
    "fra1w8s3e7v5a78623t8cq43uejtw90yzd0xctpwv63um5amtv72detq95v0dy",
    "fra1ukju0dhmx0sjwzcgjzgg3e7n6f755jkkfl9akq4hleulds9a0hgq4uzcp5",
    "fra1mjdr0mgn2e0670hxptpzu9tmf0ary8yj8nv90znjspwdupv9aacqwrg3dx",
    "fra1whn756rtqt3gpsmdlw6pvns75xdh3ttqslvxaf7eefwa83pcnlhsree9gv",
    "fra1dkn9w5c674grdl6gmvj0s8zs0z2nf39zrmp3dpq5rqnnf9axwjrqexqnd6", // foundation account
];

fn main() {
    // https://prod-mainnet.prod.findora.org:26657/tx_search?per_page=100&query=%22addr.from.fra1dkn9w5c674grdl6gmvj0s8zs0z2nf39zrmp3dpq5rqnnf9axwjrqexqnd6=%27y%27%22
}

fn get_nonconfidential_balance(txo: &BlindAssetRecord) -> Option<u64> {
    if let XfrAmount::NonConfidential(n) = txo.amount {
        Some(n)
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct Res {
    txs: Vec<Tx>,
    total_count: u64,
}

impl From<HttpRes> for Res {
    fn from(r: HttpRes) -> Self {
        todo!()
    }
}

#[derive(Clone, Debug)]
struct Tx {
    height: u64,
    successful: bool,
    tx: Transaction,
}

#[derive(Clone, Debug, Deserialize)]
struct Transaction {
    body: TransactionBody,
}

#[derive(Clone, Debug, Deserialize)]
struct TransactionBody {
    operations: Vec<Operation>,
}

#[derive(Clone, Debug, Deserialize)]
enum Operation {
    TransferAsset(TransferAsset),
}

#[derive(Clone, Debug, Deserialize)]
struct TransferAsset {
    body: TransferAssetBody,
}

#[derive(Clone, Debug, Deserialize)]
struct TransferAssetBody {
    transfer: Box<XfrBody>,
}

#[derive(Clone, Debug, Deserialize)]
struct HttpRes {
    result: HttpRet,
}

#[derive(Clone, Debug, Deserialize)]
struct HttpRet {
    txs: Vec<HttpTx>,
    total_count: String,
}

#[derive(Clone, Debug, Deserialize)]
struct HttpTx {
    height: String,
    tx_result: HttpTxResult,
    tx: String,
}

#[derive(Clone, Debug, Deserialize)]
struct HttpTxResult {
    code: u8,
}
