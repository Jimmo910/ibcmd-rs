# Export one extension from a file infobase

## Why

The native 8.3.27.2214 command exports a named file-infobase extension, but ibcmd-rs rejects this route (#520).

## What changes

- Read the measured file registry and select exactly one named extension.
- Resolve its CAS image and reuse the XML extension exporter without invoking the platform.
- Refuse unknown registry/prepared profiles, corrupt CAS and incomplete XML before publication.
- Prove own/borrowed-object XML including ConfigDumpInfo parity, two-extension isolation and database immutability on synthetic fixtures.

All-extensions, listing, import/apply, writes, SQL activation, 8.5 and EDT remain outside this change. Refs #348; neither issue closes automatically.
