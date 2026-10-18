# Security policy

## Supported versions

Only the [latest release](https://github.com/hex-dump/perihub/releases/latest) receives security fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through GitHub instead: go to the **Security** tab and click **Report a vulnerability**, or use [this link](https://github.com/hex-dump/perihub/security/advisories/new).

Include, if you can:

- the PeriHub version and your operating system;
- the device involved, if any;
- steps to reproduce and what an attacker could achieve.

PeriHub is maintained by one person in their spare time, so responses are best effort. I will acknowledge the report as soon as I can, keep you updated while a fix is prepared, and credit you in the release notes unless you prefer otherwise.

## Scope

In scope:

- anything that makes PeriHub send a command, HID++ feature, memory write or key code outside the allow-lists described in the README's [Safety](README.md#safety) section, for example in a way that could corrupt a device;
- the Linux udev rule (`packaging/linux/70-perihub.rules`) granting access to more devices or users than intended;
- key presses read for the reactive lighting effect being kept, logged or exposed in any way;
- the desktop app exposing commands or permissions beyond what its interface needs;
- the installers and release files.

Out of scope: bugs in the manufacturers' own software or device firmware, and ordinary bugs with no security impact. For those, open a normal [issue](https://github.com/hex-dump/perihub/issues/new/choose).
