# NEMESIS Name Collision Reconnaissance

**Observed:** 2026-08-20  
**Scope:** bounded public-name reconnaissance before the first GitHub release  
**Conclusion:** collisions exist; uniqueness and legal clearance are not established

This check is a discovery snapshot, not a trademark opinion. It does not determine likelihood of confusion, ownership, geographic scope, goods/services overlap, common-law use, or legal availability.

## GitHub

The target `AnubisQuantumCipher/nemesis` did not exist when queried with `gh repo view` before repository creation. A separate [GitHub repository search for `nemesis in:name`](https://api.github.com/search/repositories?q=nemesis%20in%3Aname&per_page=10) returned thousands of repositories, including established software projects using Nemesis in their names. Examples observed in the API response included:

- [`SpecterOps/Nemesis`](https://github.com/SpecterOps/Nemesis), described there as an offensive data-enrichment pipeline;
- [`libnet/nemesis`](https://github.com/libnet/nemesis), described there as a network packet crafting and injection utility.

Decision boundary: the architect locked the NEMESIS identity and explicitly authorized only the public GitHub repository `AnubisQuantumCipher/nemesis`. Repository availability does not imply name uniqueness or trademark rights.

## Package registries

Exact `nemesis` package names were already occupied when queried:

| Registry | Primary record | Observed use |
|---|---|---|
| crates.io | [`nemesis`](https://crates.io/crates/nemesis) | “Nemesis utility library,” version `0.1.0` in the registry API snapshot |
| npm | [`nemesis`](https://www.npmjs.com/package/nemesis) | JavaScript web framework, latest tag `0.1.0` in the registry document |
| PyPI | [`nemesis`](https://pypi.org/project/nemesis/) | Elasticsearch resources-as-code tool, version `0.0.9` in the PyPI JSON document |

No crates.io, npm, PyPI, Homebrew, App Store, or other package name is registered or claimed by this release. The repository's `nemesis` command names describe local source/build surfaces only.

## Domains

Registry RDAP records showed both short product domains already registered:

- [`nemesis.dev`](https://pubapi.registry.google/rdap/domain/nemesis.dev)
- [`nemesis.app`](https://pubapi.registry.google/rdap/domain/nemesis.app)

`nemesisos.com` also resolved in the DNS snapshot. No domain is purchased, reserved, or presented as the project homepage.

## Trademark databases

A bounded keyword search was attempted in the [USPTO Trademark Search system](https://tmsearch.uspto.gov/) and [WIPO Global Brand Database](https://branddb.wipo.int/). Search indexing exposed multiple candidate `NEMESIS` records, but the databases' interactive/anti-automation surfaces did not yield an exhaustive, goods-and-services-qualified review in this mission.

Therefore:

- no candidate record is characterized here as conflicting or non-conflicting;
- no jurisdictional or class analysis was performed;
- no common-law search was performed;
- no legal clearance is claimed.

## Release decision

Proceed only with the architect-authorized GitHub source repository and the bounded macOS alpha. Retain this collision record and the explicit non-claim in README, release notes, receipts, and repository metadata. Any later package registration, domain acquisition, commercial branding decision, or broader distribution requires a fresh qualified review rather than extrapolation from this snapshot.
