# Chill-rs Change Log

## v0.2.1 (unreleased)

No changes yet!

## v0.2.0 (2016-05-28)

The v0.2.0 release introduces several breaking changes, mainly for the
purpose of simplifying Chill's API.

### Breaking changes

* Replace each owning and non-owning path-related type pair (e.g.,
  `DatabaseName` and `DatabaseNameRef`) with a single owning type (e.g.,
  `DatabaseName`) ([issue #33](issue_33)). This increases the number of
  memory allocations in some cases but vastly simplifies Chill's API.
* Convert action-constructing `Client` methods to be infallible ([issue
  #34](issue_34)). This simplifies Chill's API.
* Remove some type parameters for view execution ([issue #40](issue_40)).
  This affects these types: `ExecuteView`, `ViewResponse`, `ViewRow`,
  and `ViewResponseBuilder`. This change simplifies Chill's API by
  eliminating the need for applications to explicitly specify types when
  executing a view.
* Convert `ViewResponse` from an enum to a struct and generalize for
  storing _reduced_, _grouped_, and _unreduced_ view responses ([issue
  #49](issue_49)).

### New

* New support for the `group` query parameter when executing a view.
* New support for the `group_level` query parameter when executing a
  view.

## v0.1.2 (2016-05-07)

The v0.1.2 release has a few small changes.

* New support for the `limit` query parameter when executing a view.
* Deprecate the `Document::id` method. Applications should use
  `Document::path` instead.
* Define `IntoUrl` not to be based on Hyper's trait of the same name.

## v0.1.1 (2016-04-16)

The v0.1.1 release extends Chill's coverage of the CouchDB API.

* New support for executing views (`action::ExecuteView`).
* New support for creating, reading, updating, and deleting attachments
  as part of reading and updating documents. However, Chill still has no
  support for accessing attachments individually.

## v0.1.0 (2016-03-26)

This is the first release. It has minimal support for creating, reading,
updating, and deleting documents.

[issue_33]: https://github.com/chill-rs/chill/issues/33
[issue_34]: https://github.com/chill-rs/chill/issues/34
[issue_40]: https://github.com/chill-rs/chill/issues/40
[issue_42]: https://github.com/chill-rs/chill/issues/42
[issue_49]: https://github.com/chill-rs/chill/issues/49
