# Lighthouse: Ethereum 2.0

An open-source Ethereum 2.0 client, written in Rust and maintained by Sigma Prime.

[![Build Status]][Build Link] [![Book Status]][Book Link] [![Chat Badge]][Chat Link]

[Build Status]: https://github.com/sigp/lighthouse/workflows/test-suite/badge.svg?branch=master
[Build Link]: https://github.com/sigp/lighthouse/actions
[Chat Badge]: https://img.shields.io/badge/chat-discord-%237289da
[Chat Link]: https://discord.gg/cyAszAh
[Book Status]:https://img.shields.io/badge/user--docs-master-informational
[Book Link]: https://lighthouse-book.sigmaprime.io
[stable]: https://github.com/sigp/lighthouse/tree/stable
[unstable]: https://github.com/sigp/lighthouse/tree/unstable

[Documentation](https://lighthouse-book.sigmaprime.io)

![Banner](https://i.postimg.cc/hjdTGKPd/photo-2020-10-23-09-52-16.jpg)

## Overview

Lighthouse is:

- Ready for use on Eth2 mainnet.
- Fully open-source, licensed under Apache 2.0.
- Security-focused. Fuzzing has begun and security reviews are underway.
- Built in [Rust](https://www.rust-lang.org), a modern language providing unique safety guarantees and
	excellent performance (comparable to C++).
- Funded by various organisations, including Sigma Prime, the
	Ethereum Foundation, ConsenSys and private individuals.
- Actively involved in the specification and security analysis of the
	Ethereum 2.0 specification.

## Eth2 Deposit Contract

The Lighthouse team acknowledges
[`0x00000000219ab540356cBB839Cbe05303d7705Fa`](https://etherscan.io/address/0x00000000219ab540356cbb839cbe05303d7705fa)
as the canonical Eth2 deposit contract address.

## Documentation

The [Lighthouse Book](https://lighthouse-book.sigmaprime.io) contains information
for testnet users and developers.

If you'd like some background on Sigma Prime, please see the [Lighthouse Update
\#00](https://lighthouse.sigmaprime.io/update-00.html) blog post or
[sigmaprime.io](https://sigmaprime.io).

## Branches

Lighthouse maintains two permanent branches:

- [`stable`][stable]: Always points to the latest stable release.
  - This is ideal for most users.
- [`unstable`][unstable]: Used for development, contains the latest PRs.
  - Developers should base thier PRs on this branch.

## Contributing

Lighthouse welcomes contributors.

If you are looking to contribute, please head to the
[Contributing](https://lighthouse-book.sigmaprime.io/contributing.html) section
of the Lighthouse book.

## Contact

The best place for discussion is the [Lighthouse Discord
server](https://discord.gg/cyAszAh). Alternatively, you may use the
[sigp/lighthouse gitter](https://gitter.im/sigp/lighthouse).

Sign up to the [Lighthouse Development Updates](https://mailchi.mp/3d9df0417779/lighthouse-dev-updates)
mailing list for email notifications about releases, network status and other important information.

Encrypt sensitive messages using our [PGP
key](https://keybase.io/sigp/pgp_keys.asc?fingerprint=dcf37e025d6c9d42ea795b119e7c6cf9988604be).

## Donations

Lighthouse is an open-source project and a public good. Funding public goods is
hard and we're grateful for the donations we receive from the community via:

- [Gitcoin Grants](https://gitcoin.co/grants/25/lighthouse-ethereum-20-client).
- Ethereum address: `0x25c4a76E7d118705e7Ea2e9b7d8C59930d8aCD3b` (donation.sigmaprime.eth).
