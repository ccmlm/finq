# finq

Querying the recent transactions of a set of findora addresses.

## Usage

```shell
finq 0.1.0

USAGE:
    finq [OPTIONS]

OPTIONS:
    -d, --days-within <DAYS_WITHIN>              Optional, span of recent days, default to 7
    -h, --help                                   Print help information
    -r, --recursive-depth <RECURSIVE_DEPTH>
    -t, --target-addr-list <TARGET_ADDR_LIST>    Optional, default to the 9 reserved addresses
    -V, --version                                Print version information
```

## Example 1

```shell
make release
./target/release/finq -d 100 -r 1
```

**Outputs:**

```
report = [
    ReceiverSet {
        total_cnt: 3,
        confidential_cnt: 0,
        non_confidential_amount_readable: "82111181.23781",
        entries: [
            Receiver {
                addr: "fra1dkn9w5c674grdl6gmvj0s8zs0z2nf39zrmp3dpq5rqnnf9axwjrqexqnd6",
                kind: Reserved,
                total_cnt: 1,
                confidential_cnt: 0,
                non_confidential_amount: 945000000000000,
                non_confidential_amount_readable: "945000000",
            },
            Receiver {
                addr: "fra1whn756rtqt3gpsmdlw6pvns75xdh3ttqslvxaf7eefwa83pcnlhsree9gv",
                kind: Reserved,
                total_cnt: 1,
                confidential_cnt: 0,
                non_confidential_amount: 944999999990000,
                non_confidential_amount_readable: "944999999.99",
            },
            Receiver {
                addr: "fra147zqdev23e3etfegvvjznkcgk7ch0aax0txgxweq8aq0mrzv5rcswh52k2",
                kind: Normal,
                total_cnt: 1,
                confidential_cnt: 0,
                non_confidential_amount: 82111181237810,
                non_confidential_amount_readable: "82111181.23781",
            },
        ],
    },
]
```

## Example 2

```shell
make release
./target/release/finq -d 100 -r 2
```

**Outputs:** [**example.log**](./example.log)
