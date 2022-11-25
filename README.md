# Bitcoin Canister

<div>
  <p>
    <a href="https://github.com/dfinity/bitcoin-canister/blob/master/LICENSE"><img alt="Apache-2.0" src="https://img.shields.io/github/license/dfinity/bitcoin-canister"/></a>
    <a href="https://internetcomputer.org/docs/current/references/ic-interface-spec#ic-bitcoin-api"><img alt="API Specification" src="https://img.shields.io/badge/spec-interface%20specification-blue"/></a>
    <a href="https://forum.dfinity.org/"><img alt="Chat on the Forum" src="https://img.shields.io/badge/help-post%20on%20forum.dfinity.org-yellow"></a>
  </p>
</div>

## Overview
The Bitcoin canister is the core component of the Bitcoin integration project. It enables other canisters deployed on the Internet Computer to use Bitcoin and interact with the Bitcoin network.

To this end, it provides a low-level API with a small set of functions, which serve as the foundation to build powerful Bitcoin libraries and other development tools, and Bitcoin smart contracts running on the Internet Computer.

## Useful Links

* [Documentation](https://internetcomputer.org/docs/current/developer-docs/integrations/bitcoin/)
* [Interface Specification](https://internetcomputer.org/docs/current/references/ic-interface-spec#ic-bitcoin-api)
* [Tutorial: Deploying Your First Bitcoin Dapp](https://internetcomputer.org/docs/current/samples/deploying-your-first-bitcoin-dapp/)
* [Tutorial: Developing Bitcoin Dapps Locally](https://internetcomputer.org/docs/current/developer-docs/integrations/bitcoin/local-development)

## Disclaimer

The Bitcoin canister is still in beta and does not yet implement the specification fully. The missing functionality includes (but is not limited to):

* Block validation
* The wait-for-quiet mechanism
