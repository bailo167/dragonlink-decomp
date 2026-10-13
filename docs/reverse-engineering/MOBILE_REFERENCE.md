# Official mobile reference software

Official Product Madness (Aristocrat) social-casino apps that include Dragon Link content.
Read 2026-10-13 from the Australian storefront listings. **Nothing has been downloaded or inspected
yet**; everything here is from store listings, read once (STRONG EVIDENCE at most).

Mobile software is evidence for **presentation**, not for cabinet mathematics
([../EVIDENCE.md](../EVIDENCE.md), rule 4). Whether it shares any mathematics with the cabinet games
is UNKNOWN, and is one of the questions the mathematics lane sets out to answer
([math/README.md](math/README.md)).

## Apps

All four are published under the developer "Product Madness", free to download, and described as
virtual-coin social casinos with no real-money play.

| App | Android package | What the listing says | Which of the four titles |
|---|---|---|---|
| **Mighty Fu Casino** (formerly FaFaFa Gold) | `com.productmadness.fafafagold` | "Dragon Link pokies featuring Golden Century, Happy & Prosperous, Autumn Moon, Panda Magic, Golden Gong slot games" | **All four named** |
| Lightning Link Casino | `com.productmadness.lightninglink` | Google Play: "Dragon Link Hold & Spin pokies". Apple AU listing: "Golden Century & Panda Magic – Classic Dragon Link fun" | Golden Century and Panda Magic named; the other two not confirmed |
| Cashman Casino | `com.productmadness.cashmancasino` | Dragon Link named generically | none confirmed |
| Heart of Vegas | `com.productmadness.hovmobile` | Dragon Link named generically | none confirmed |

Listings: `https://play.google.com/store/apps/details?id=<package>`.

**Mighty Fu Casino is the first acquisition target**: it is the only listing that names all four titles.

## Not established

- Whether each title is actually present in each app (listings only; not checked in-app).
- The engine. Job postings describe Product Madness client roles as Unity/C#, which does not tie
  Unity to a particular app. UNVERIFIED LEAD.
- Whether game content ships inside the package or is downloaded after launch, possibly behind a
  login. This decides how much can be inventoried without an account.
- Whether mobile assets, timing or mathematics match any cabinet build.

## Acquisition plan

1. Obtain the package from the official store onto a device or emulator the owner controls, and
   copy it out with the platform's own tools. Not from a mirror site.
2. Place it unmodified under `official-mobile/<package>/<version>/` in the private source root;
   record the version, the date, the channel and the SHA-256.
3. Inventory it (`dragonlink-inventory`), identify the engine and containers, and record the
   findings here and in [ASSET_FORMATS](README.md) when that document exists.
4. Study presentation first. Any mathematics found is recorded as *mobile* mathematics and compared
   with cabinet evidence only when cabinet evidence exists.

**This step needs the owner**: installing from the store requires a store account on a device or
emulator. It is listed in [../BLOCKERS.md](../BLOCKERS.md).
