#![allow(dead_code)]

use {
    bech32::ToBase32,
    clap::Parser,
    once_cell::sync::Lazy,
    ruc::*,
    serde::{de::DeserializeOwned, Deserialize},
    std::{collections::HashSet, env},
    zei::{
        serialization::ZeiFromToBytes,
        xfr::{
            sig::XfrPublicKey,
            structs::{
                AssetType, BlindAssetRecord, OwnerMemo, XfrAmount, XfrAssetType, XfrBody,
                ASSET_TYPE_LENGTH,
            },
        },
    },
};

fn main() {
    let args = Args::parse();

    if args.localhost {
        env::set_var("FINQ_SERVER_URL", "http://localhost");
    }

    let mut recursive_depth = args.recursive_depth.unwrap_or(2);
    let days_within = args.days_within.unwrap_or(7);

    let mut addr_list = if let Some(l) = args.target_addr_list {
        l
    } else {
        ADDR_LIST.iter().map(|a| a.to_string()).collect()
    };
    addr_list.sort_unstable();
    addr_list.dedup();

    if addr_list.is_empty() {
        eprintln!("\x1b[31;1mAddress list is empty!\x1b[0m");
        return;
    }

    let mut hist = HashSet::new();
    let mut report = Report::default();
    pnk!(trace(
        &mut report,
        addr_list,
        days_within,
        &mut hist,
        &mut recursive_depth
    ));

    report_make_readable(&mut report);

    dbg!(report);
}

fn trace(
    report: &mut Report,
    addr_list: Vec<FraAddr>,
    days_within: u64,
    hist: &mut HashSet<FraAddr>,
    recursive_depth: &mut u8,
) -> Result<()> {
    if 0 == *recursive_depth {
        return Ok(());
    }
    *recursive_depth -= 1;

    let al = addr_list.into_iter().fold(vec![], |mut acc, new| {
        if !hist.contains(&new) {
            acc.push(new);
        }
        acc
    });

    let mut res = map! {};
    let mut next_round = HashSet::new();
    for addr in al.into_iter() {
        let l = get_tx_list(&addr, days_within).c(d!())?;
        for ops in l.into_iter().map(|tx| tx.tx.body.operations) {
            for o in ops.into_iter() {
                macro_rules! op {
                    ($output: expr) => {
                        let receiver = pubkey_to_bech32(&$output.public_key);
                        alt!(addr == receiver, continue);
                        next_round.insert(receiver.clone());
                        let (confidential_cnt, am) =
                            if let Some(am) = get_nonconfidential_balance(&$output) {
                                (0, am)
                            } else {
                                (1, 0)
                            };
                        let en = res.entry($output.public_key).or_insert(Receiver {
                            addr: receiver,
                            kind: gen_kind(&$output),
                            total_cnt: 0,
                            confidential_cnt: 0,
                            non_confidential_amount: 0,
                            non_confidential_amount_readable: String::new(),
                        });
                        en.total_cnt += 1;
                        en.confidential_cnt += confidential_cnt;
                        en.non_confidential_amount += am;
                    };
                }
                match o {
                    Operation::TransferAsset(o) => {
                        for output in o.body.transfer.outputs.into_iter() {
                            op!(output);
                        }
                    }
                    Operation::IssueAsset(o) => {
                        for output in o.body.records.into_iter().map(|r| r.0.record) {
                            op!(output);
                        }
                    }
                    _ => (),
                };
            }
        }
        hist.insert(addr);
    }

    let (total_cnt, confidential_cnt, mut entries) =
        res.into_iter().fold((0, 0, vec![]), |mut acc, (_, new)| {
            acc.0 += 1;
            acc.1 += new.confidential_cnt;
            acc.2.push(new);
            acc
        });

    entries.sort_unstable_by(|a, b| b.non_confidential_amount.cmp(&a.non_confidential_amount));

    report.push(ReceiverSet {
        total_cnt,
        confidential_cnt,
        non_confidential_amount_readable: String::new(),
        entries,
    });

    let next_round = if next_round.is_empty() {
        return Ok(());
    } else {
        next_round.into_iter().collect()
    };

    trace(report, next_round, days_within, hist, recursive_depth)
}

fn is_fee_or_burn(output: &BlindAssetRecord) -> bool {
    output.public_key == *BH_PK
}

fn is_staking_or_evm_conversion(output: &BlindAssetRecord) -> bool {
    output.public_key == *BH_PK_STAKING
}

fn is_reserved(output: &BlindAssetRecord) -> bool {
    ADDR_LIST.contains(&pubkey_to_bech32(&output.public_key).as_str())
}

fn gen_kind(o: &BlindAssetRecord) -> AddrKind {
    if is_fee_or_burn(o) {
        AddrKind::FeeOrBurn
    } else if is_staking_or_evm_conversion(o) {
        AddrKind::StakingOrEvmConversion
    } else if is_reserved(o) {
        AddrKind::Reserved
    } else {
        AddrKind::Normal
    }
}

// fn height_to_days_ago(h: Height, latest_h: Height) -> u64 {
//     latest_h.saturating_sub(h) * BLOCK_ITV_SECS / (24 * 3600)
// }

fn days_to_start_height(mut days_within: u64) -> Height {
    if 0 == days_within {
        days_within = 1;
    }
    days_within * 24 * 3600 / BLOCK_ITV_SECS
}

fn get_nonconfidential_balance(output: &BlindAssetRecord) -> Option<Amount> {
    if let XfrAssetType::NonConfidential(ty) = output.asset_type {
        if ASSET_TYPE_FRA == ty {
            if let XfrAmount::NonConfidential(n) = output.amount {
                return Some(n);
            }
        }
    }
    None
}

fn get_tx_list(addr: FraAddrRef, days_within: u64) -> Result<TxList> {
    let start_height = (*LATEST_HEIGHT).saturating_sub(days_to_start_height(days_within));
    let url = format!(
        r#"{}:26657/tx_search?per_page=100&query="addr.from.{}='y'"&order_by="desc""#,
        &*SERVER_URL, addr
    );
    let res = http_get::<HttpRes>(&url).c(d!())?;
    let total_cnt = res.result.total_count.parse::<usize>().unwrap();
    let mut res: TxList = res.into();
    if 0 < total_cnt {
        let mut page = 1;
        let mut received_cnt = res.len();
        let mut min_height = res.last().unwrap().height;
        while start_height < min_height && total_cnt > received_cnt {
            let url = format!("{}&page={}", &url, 1 + page);
            if let Ok(part) = info!(http_get::<HttpRes>(&url)) {
                let part: TxList = part.into();
                res.extend_from_slice(&part);
                page += 1;
                received_cnt = res.len();
                min_height = res.last().unwrap().height;
            } else {
                break;
            }
        }
    }

    assert!(res.len() <= total_cnt);

    Ok(res
        .into_iter()
        .filter(|r| r.height > start_height)
        .collect())
}

fn get_latest_height() -> Result<Height> {
    #[derive(Deserialize)]
    struct Res {
        result: Ret,
    }
    #[derive(Deserialize)]
    struct Ret {
        block_height: String,
    }
    let url = format!("{}:26657/validators?per_page=1", &*SERVER_URL);
    http_get::<Res>(&url)
        .c(d!())
        .and_then(|r| r.result.block_height.parse::<Height>().c(d!()))
}

fn http_get<T: DeserializeOwned>(url: &str) -> Result<T> {
    attohttpc::get(url).send().c(d!())?.json::<T>().c(d!())
}

fn pubkey_to_bech32(key: &XfrPublicKey) -> FraAddr {
    bech32enc(&XfrPublicKey::zei_to_bytes(key))
}

fn bech32enc<T: AsRef<[u8]> + ToBase32>(input: &T) -> FraAddr {
    bech32::encode("fra", input.to_base32()).unwrap()
}

// fn pubkey_from_bech32(addr: FraAddrRef) -> Result<XfrPublicKey> {
//     bech32dec(addr)
//         .c(d!())
//         .and_then(|bytes| XfrPublicKey::zei_from_bytes(&bytes).c(d!()))
// }
//
// fn bech32dec(input: FraAddrRef) -> Result<Vec<u8>> {
//     bech32::decode(input)
//         .c(d!())
//         .and_then(|(_, data)| Vec::<u8>::from_base32(&data).c(d!()))
// }

const PK_LEN: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;
const BH_PK_BYTES: [u8; PK_LEN] = [0; PK_LEN];
const BH_PK_STAKING_BYTES: [u8; PK_LEN] = [1; PK_LEN];

const ASSET_TYPE_FRA: AssetType = AssetType([0; ASSET_TYPE_LENGTH]);
const FRA_DECIMALS: u32 = 6;

const BLOCK_ITV_SECS: u64 = 16;

static BH_PK: Lazy<XfrPublicKey> =
    Lazy::new(|| pnk!(XfrPublicKey::zei_from_bytes(&BH_PK_BYTES[..])));
static BH_PK_STAKING: Lazy<XfrPublicKey> =
    Lazy::new(|| pnk!(XfrPublicKey::zei_from_bytes(&BH_PK_STAKING_BYTES[..])));
static SERVER_URL: Lazy<String> = Lazy::new(|| {
    env::var("FINQ_SERVER_URL")
        .unwrap_or_else(|_| "https://prod-mainnet.prod.findora.org".to_owned())
});

static LATEST_HEIGHT: Lazy<Height> = Lazy::new(|| pnk!(get_latest_height()));

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

type Report = Vec<ReceiverSet>;

fn report_make_readable(r: &mut Report) {
    r.iter_mut().for_each(|r| {
        let (tc, cc, non_c_am) = r.entries.iter().fold((0, 0, 0), |acc, new| {
            (
                acc.0 + new.total_cnt,
                acc.1 + new.confidential_cnt,
                acc.2 + new.non_confidential_amount,
            )
        });
        r.total_cnt = tc;
        r.confidential_cnt = cc;
        r.non_confidential_amount_readable = to_float_str(non_c_am);
        r.entries.iter_mut().for_each(|r| {
            r.non_confidential_amount_readable = to_float_str(r.non_confidential_amount);
        })
    });
}

#[derive(Default, Debug)]
struct ReceiverSet {
    total_cnt: u64,
    confidential_cnt: u64,
    non_confidential_amount_readable: String,
    entries: Vec<Receiver>,
}

#[derive(Debug)]
struct Receiver {
    addr: FraAddr,
    kind: AddrKind,
    total_cnt: u64,
    confidential_cnt: u64,
    non_confidential_amount: Amount,
    non_confidential_amount_readable: String,
}

fn to_float_str(n: u64) -> String {
    let i = n / 10_u64.pow(FRA_DECIMALS);
    let j = n - i * 10_u64.pow(FRA_DECIMALS);
    (i.to_string() + "." + j.to_string().trim_end_matches('0'))
        .trim_end_matches('.')
        .to_owned()
}

#[derive(Debug)]
enum AddrKind {
    Normal,
    FeeOrBurn,
    StakingOrEvmConversion,
    Reserved,
}

type Height = u64;
type Amount = u64;
type FraAddr = String;
type FraAddrRef<'a> = &'a str;

type TxList = Vec<Tx>;

impl From<HttpRes> for TxList {
    fn from(t: HttpRes) -> Self {
        t.result
            .txs
            .into_iter()
            .filter(|tx| 0 == tx.tx_result.code)
            .map(|tx| tx.into())
            .collect()
    }
}

#[derive(Clone, Debug)]
struct Tx {
    height: Height,
    tx: Transaction,
}

impl From<HttpTx> for Tx {
    fn from(t: HttpTx) -> Self {
        let height = t.height.parse::<Height>().unwrap();
        let tx = base64::decode(&t.tx).unwrap();
        let tx = serde_json::from_slice::<Transaction>(&tx).unwrap_or_default();
        Tx { height, tx }
    }
}

#[derive(Clone, Default, Debug, Deserialize)]
struct Transaction {
    body: TransactionBody,
}

#[derive(Clone, Default, Debug, Deserialize)]
struct TransactionBody {
    operations: Vec<Operation>,
}

#[derive(Clone, Debug, Deserialize)]
enum Operation {
    TransferAsset(TransferAsset),
    IssueAsset(IssueAsset),
    #[serde(skip)]
    DefineAsset,
    #[serde(skip)]
    UpdateMemo,
    #[serde(skip)]
    UpdateStaker,
    Delegation(DelegationOps),
    #[serde(skip)]
    UnDelegation,
    #[serde(skip)]
    Claim,
    #[serde(skip)]
    UpdateValidator,
    #[serde(skip)]
    Governance,
    #[serde(skip)]
    FraDistribution,
    #[serde(skip)]
    MintFra,
    #[serde(skip)]
    ConvertAccount(ConvertAccount),
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
struct IssueAsset {
    body: IssueAssetBody,
}

#[derive(Clone, Debug, Deserialize)]
struct IssueAssetBody {
    records: Vec<(TxOutput, Option<OwnerMemo>)>,
}

#[derive(Clone, Debug, Deserialize)]
struct DelegationOps {}

// #[derive(Clone, Debug, Deserialize)]
// struct MintFraOps {
//     entries: Vec<MintEntry>,
// }
//
// #[derive(Clone, Debug, Deserialize)]
// struct MintEntry {
//     utxo: TxOutput,
// }

#[derive(Clone, Debug, Deserialize)]
struct TxOutput {
    record: BlindAssetRecord,
}

#[derive(Clone, Debug, Deserialize)]
struct ConvertAccount {}

#[derive(Deserialize)]
struct HttpRes {
    result: HttpRet,
}

#[derive(Deserialize)]
struct HttpRet {
    txs: Vec<HttpTx>,
    total_count: String,
}

#[derive(Deserialize)]
struct HttpTx {
    height: String,
    tx_result: HttpTxResult,
    tx: String,
}

#[derive(Deserialize)]
struct HttpTxResult {
    code: u64,
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long, help = "Optional, span of recent days, default to 7")]
    days_within: Option<u64>,
    #[clap(short, long)]
    recursive_depth: Option<u8>,
    #[clap(short, long, help = "Optional, default to the 9 reserved addresses")]
    target_addr_list: Option<Vec<FraAddr>>,
    #[clap(short, long, help = "Use `http://localhost` as 'SERVER_URL'")]
    localhost: bool,
}
