# CI: build automatica dell'APK Android

Stato: proposta, da implementare. Segue
`specs/M6/2026-09-04-prima-build-android-reale.DONE.md` — quella spec
ha prodotto una ricetta di build verificata per davvero (non solo
progettata), questa la automatizza. Decisioni confermate dall'utente:
runner `ubuntu-latest`, trigger solo su tag di release, solo ABI
arm64 (`aarch64`).

## Perché solo su tag, non su ogni push/PR

`test.yml` (`specs/release/2026-09-03-ci.TODO.md`) gira su ogni
push/PR e oggi impiega ~1-3 minuti. Una build Android (SDK, NDK,
Gradle, compilazione Rust per un target nuovo) aggiunge diversi
minuti — non vale rallentare ogni giro di test per un artefatto che
serve solo al momento di una release. Stesso principio già usato per
non scrivere `release.yml` (desktop) finché la firma del codice non è
risolta: qui non c'è quel blocco (un APK di debug non richiede
certificati), quindi si può scrivere subito.

## Perché solo arm64

Copre la stragrande maggioranza dei device Android reali in uso oggi.
Build "universal" (4 ABI) quadruplica il tempo di compilazione Rust
per un beneficio marginale (emulatori x86/device molto vecchi) — se
servisse in futuro, si allarga la matrice `--target`, non è una
riscrittura.

## Modifiche

Nuovo `.github/workflows/android.yml`:

```yaml
name: android

on:
  push:
    tags:
      - "v*"
  workflow_dispatch: {}

jobs:
  build-apk:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android

      - uses: Swatinem/rust-cache@v2

      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: "17"

      - uses: android-actions/setup-android@v3

      - name: Install NDK
        run: sdkmanager --install "ndk;28.2.13676358"

      - name: Set NDK cross-compile env vars
        # openssl-sys (via git2) non trova i binutils prefissati per
        # tripla che NDK r23+ non fornisce più — stessa causa e stesso
        # fix scoperti nella build locale, vedi
        # specs/M6/2026-09-04-prima-build-android-reale.DONE.md.
        run: |
          NDK_BIN="$ANDROID_HOME/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin"
          echo "AR_aarch64_linux_android=$NDK_BIN/llvm-ar" >> "$GITHUB_ENV"
          echo "RANLIB_aarch64_linux_android=$NDK_BIN/llvm-ranlib" >> "$GITHUB_ENV"

      - run: npm ci

      - name: Build Android APK (debug)
        run: npx tauri android build --debug --target aarch64 --apk

      - uses: actions/upload-artifact@v4
        with:
          name: ramus-android-apk
          path: src-tauri/gen/android/app/build/outputs/apk/**/*.apk
```

Nuove GitHub Action non ufficiali ma standard de facto, motivate qui
(infrastruttura, non codice applicativo): `android-actions/setup-android`
(installa Android SDK cmdline-tools sul runner — alternativa a
scaricare/accettare licenze a mano) e `actions/setup-java` (ufficiale
GitHub). `dtolnay/rust-toolchain`/`Swatinem/rust-cache` già motivate e
in uso in `test.yml`.

**`--debug`, non una build firmata per il Play Store**: nessun
certificato di firma richiesto (Android genera una chiave di debug
automatica), coerente con l'assenza di una decisione di firma presa
finora — stessa dipendenza già documentata in
`specs/release/2026-09-03-firma-notarizzazione.TODO.md` (lì per
macOS/Windows, la stessa domanda "vuoi investire in certificati" vale
anche per una release reale sul Play Store, non affrontata qui).

## Fuori scope

- Build firmata per il Play Store: dipende da una decisione di firma
  non ancora presa (vedi sopra).
- Pubblicazione automatica dell'APK su una GitHub Release (upload
  come release asset invece che solo come artifact del workflow): un
  passo in più ragionevole una volta che esiste `release.yml`
  desktop — non scritto ancora, stesso motivo.
- iOS: non tentato nemmeno nella build locale, nessuna pipeline CI
  possibile prima di quello.
- Build per gli altri 3 ABI (`armv7`/`i686`/`x86_64`): confermato
  fuori scope dall'utente, si allarga la matrice se servisse.

## Verifica

YAML validato, workflow eseguito per davvero via
`workflow_dispatch` (non solo scritto e mai fatto girare) prima di
chiudere questa spec — vedi sotto per l'esito reale.
