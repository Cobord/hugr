# Changelog

## [0.2.0](https://github.com/Cobord/hugr/compare/hugr-py-v0.1.0...hugr-py-v0.2.0) (2024-05-07)


### ⚠ BREAKING CHANGES

* serialisation format
* serialisation schema
* serialisation schema ([#968](https://github.com/Cobord/hugr/issues/968))
* bring back Value ([#967](https://github.com/Cobord/hugr/issues/967))
* Flatten `LeafOp` ([#922](https://github.com/Cobord/hugr/issues/922))
* EdgeKind::{Static -> Const}, add new EdgeKind::Function, Type contains only monomorphic functions, remove TypeApply.
* **py:** Rename package to `hugr` ([#913](https://github.com/Cobord/hugr/issues/913))

### Features

* Add LoadFunction node ([#947](https://github.com/Cobord/hugr/issues/947)) ([81e9602](https://github.com/Cobord/hugr/commit/81e9602a47eddadc1c11d74ca7bda3b194d24f00))
* bring back Value ([#967](https://github.com/Cobord/hugr/issues/967)) ([0c354b6](https://github.com/Cobord/hugr/commit/0c354b6e07ae1aafee17e412fe54f7b3db321beb))
* Encoder metadata in serialized hugr ([#955](https://github.com/Cobord/hugr/issues/955)) ([0a44d48](https://github.com/Cobord/hugr/commit/0a44d487b73f58674eb5884c72479a03e924bef0))
* Flatten `LeafOp` ([#922](https://github.com/Cobord/hugr/issues/922)) ([3598913](https://github.com/Cobord/hugr/commit/3598913002a361d487aa2f6ba899739d9a3c7f13))
* No polymorphic closures ([#906](https://github.com/Cobord/hugr/issues/906)) ([b05dd6b](https://github.com/Cobord/hugr/commit/b05dd6b1a15aefee277d4034ed07039a259261e0))
* **py:** Rename package to `hugr` ([#913](https://github.com/Cobord/hugr/issues/913)) ([9fe65db](https://github.com/Cobord/hugr/commit/9fe65db9dd7fd8eee28c13e6abe71fd5cf05f90a))


### Bug Fixes

* input_port_types and other helper functions on pydantic schema ([#958](https://github.com/Cobord/hugr/issues/958)) ([8651839](https://github.com/Cobord/hugr/commit/86518390296bd93ca2fc65eccf158e21625b9073))
* Remove insert_port_types for LoadFunction ([#993](https://github.com/Cobord/hugr/issues/993)) ([acca7bf](https://github.com/Cobord/hugr/commit/acca7bfb4a074c7feb3b4b5758f589941632bc5a))
* serialisation fixes ([#997](https://github.com/Cobord/hugr/issues/997)) ([9ce6e49](https://github.com/Cobord/hugr/commit/9ce6e49d1d0c8c200b9b78ebe35a0a3257009ca1))
* serialisation schema ([#968](https://github.com/Cobord/hugr/issues/968)) ([d913f40](https://github.com/Cobord/hugr/commit/d913f406478a9f884bffef2002a02d423796b4e9))
* Set default value for Conditional.sum_rows ([#934](https://github.com/Cobord/hugr/issues/934)) ([d69198e](https://github.com/Cobord/hugr/commit/d69198eb57bf77f32538e1ba8de1f308815a067d))


### Tests

* test roundtrip serialisation against strict + lax schema ([#982](https://github.com/Cobord/hugr/issues/982)) ([954b2cb](https://github.com/Cobord/hugr/commit/954b2cb4e18903b43c6eadc5a5d9f0e0d40d56e5))

## 0.1.0 (2024-04-15)

This first release includes a pydantic model for the hugr serialization format version 1.

### Features

* Flatten `LeafOp` ([#922](https://github.com/CQCL/hugr/issues/922)) ([3598913](https://github.com/CQCL/hugr/commit/3598913002a361d487aa2f6ba899739d9a3c7f13))
* No polymorphic closures ([#906](https://github.com/CQCL/hugr/issues/906)) ([b05dd6b](https://github.com/CQCL/hugr/commit/b05dd6b1a15aefee277d4034ed07039a259261e0))
* **py:** Rename package to `hugr` ([#913](https://github.com/CQCL/hugr/issues/913)) ([9fe65db](https://github.com/CQCL/hugr/commit/9fe65db9dd7fd8eee28c13e6abe71fd5cf05f90a))
