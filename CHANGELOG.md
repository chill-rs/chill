# Chill-rs Change Log

## v0.4.0 (unreleased)

### Breaking changes

* The `testing::DocumentBuilder::build_content` method has been removed
  because its interface is not supported in Serde v1.

## v0.3.0 (2016-10-01)

The v0.3.0 release updates the `serde` dependency to version 0.8.

## v0.2.1 (2016-06-18)

The v0.2.1 release adds a few things to crate's API and includes a big
under-the-hood change that should not affect applications.

### New

* There is new support for the `include_docs` query parameter when
  executing a view ([#19](issue_19)). This allows applications to
  receive documents as part of a view response.

* There are new types to help applications when working with design
  documents: `Design`, `DesignBuilder`, and `ViewFunction`
  ([#17](issue_17)).

* There is a new type to help applications with constructing mock
  documents for testing: `DocumentBuilder`.

* All path types (e.g., `DatabasePath`, `DocumentPath`, etc.) now
  implement the `Display` trait ([#54](issue_54)).

* The `Client` type now implements the `Debug` trait ([#53](issue_53)).

### Notes

* The transport layer has been rewritten ([#51](issue_51)). The new
  transport is more generic and should make it easier to support
  asynchronous actions in the future.

## v0.2.0 (2016-05-28)

The v0.2.0 release introduces several breaking changes, mainly for the
purpose of simplifying Chill's API.

### Breaking changes

* All pairs of owning and non-owning path-related types have been
  replaced with a single owning type (e.g., `DatabaseName` and
  `DatabaseNameRef` have been replaced with a single `DatabaseName`
  type) ([#33](issue_33)). This change increases the number of
  heap-memory allocations in some cases but vastly simplifies Chill's
  API.

* All action-constructing `Client` methods are now infallible
  ([#34](issue_34)). This change simplifies Chill's API.

* Some of the type parameters for view execution have been removed
  ([#40](issue_40)). This affects these types: `ExecuteView`,
  `ViewResponse`, `ViewRow`, and `ViewResponseBuilder`. This change
  simplifies Chill's API by eliminating the need for applications to
  explicitly specify types when executing a view.

* The `ViewResponse` type has been converted from an enum to a struct
  and is now generalized for storing _reduced_, _grouped_, and
  _unreduced_ view responses ([#49](issue_49)).

### New

* There is new support for the `group` query parameter when executing a
  view ([#23](issue_23)).

* There is new support for the `group_level` query parameter when
  executing a view ([#24](issue_24)).

## v0.1.2 (2016-05-07)

The v0.1.2 release has a few small changes.

* There is new support for the `limit` query parameter when executing a
  view.

* The `Document::id` method is now deprecated. Applications should use
  `Document::path` instead.

* The `IntoUrl` trait is no longer based on Hyper's trait of the same
  name.

## v0.1.1 (2016-04-16)

The v0.1.1 release extends Chill's coverage of the CouchDB API.

* There is new support for executing views (`action::ExecuteView`).

* There is new support for creating, reading, updating, and deleting
  attachments as part of reading and updating documents. However, Chill
  still has no support for accessing attachments individually.

## v0.1.0 (2016-03-26)

This is the first release. It has minimal support for creating, reading,
updating, and deleting documents.

[issue_17]: https://github.com/chill-rs/chill/issues/17
[issue_19]: https://github.com/chill-rs/chill/issues/19
[issue_23]: https://github.com/chill-rs/chill/issues/23
[issue_24]: https://github.com/chill-rs/chill/issues/24
[issue_33]: https://github.com/chill-rs/chill/issues/33
[issue_34]: https://github.com/chill-rs/chill/issues/34
[issue_40]: https://github.com/chill-rs/chill/issues/40
[issue_42]: https://github.com/chill-rs/chill/issues/42
[issue_49]: https://github.com/chill-rs/chill/issues/49
[issue_51]: https://github.com/chill-rs/chill/issues/51
[issue_53]: https://github.com/chill-rs/chill/issues/53
[issue_54]: https://github.com/chill-rs/chill/issues/54
