<p align="center">
  <img src="app/src-tauri/icons/128x128.png" width="96" alt="">
</p>

<h1 align="center">PeriHub</h1>

<p align="center">
  Un'app open source per configurare le periferiche da gioco su <b>Windows e Linux</b>,<br>
  senza SteelSeries GG né Logitech G HUB.
</p>

<p align="center">
  <a href="https://github.com/hex-dump/perihub/releases/latest"><img src="https://img.shields.io/github/v/release/hex-dump/perihub?label=release" alt="Release"></a>
  <a href="https://github.com/hex-dump/perihub/actions/workflows/ci.yml"><img src="https://github.com/hex-dump/perihub/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/hex-dump/perihub" alt="Licenza MIT"></a>
</p>

<p align="center"><a href="README.md">English</a> · <b>Italiano</b></p>

![Illuminazione della tastiera](docs/screenshots/keyboard-lighting.png)

Per ora PeriHub supporta la tastiera **SteelSeries Apex 3 TKL** e il mouse **Logitech G502 HERO**. L'obiettivo è diventare un'unica app leggera e indipendente dai produttori per molti altri dispositivi: vedi [Hardware in arrivo](#hardware-in-arrivo). L'interfaccia è in inglese.

## Indice

- [Funzionalità](#funzionalità)
- [Hardware supportato](#hardware-supportato)
- [Hardware in arrivo](#hardware-in-arrivo)
- [Download e installazione](#download-e-installazione)
- [Come si usa](#come-si-usa)
- [Sicurezza](#sicurezza)
- [Domande frequenti](#domande-frequenti)
- [Limiti noti](#limiti-noti)
- [Compilare dai sorgenti](#compilare-dai-sorgenti)
- [Come è fatto](#come-è-fatto)
- [Contribuire](#contribuire)
- [Sostieni il progetto](#sostieni-il-progetto)
- [Riconoscimenti e licenza](#riconoscimenti-e-licenza)

## Funzionalità

### SteelSeries Apex 3 TKL

| Funzione | Dettagli | Dove viene salvata |
|---|---|---|
| Illuminazione a 8 zone | Statico (un colore per zona), sfumatura, respiro, ciclo colori, onda, arcobaleno integrato, spento | PC (PeriHub la riapplica a ogni collegamento) |
| Luminosità | 0–100% in 17 passi, sincronizzata con SteelSeries + F11/F12 | PC |
| Rimappatura tasti | Ogni tasto, sul livello normale e su quello «tasto SteelSeries premuto», con layout ANSI e ISO | **Memoria della tastiera**: funziona anche senza PeriHub |

### Logitech G502 HERO

| Funzione | Dettagli | Dove viene salvata |
|---|---|---|
| Profili | 5 profili in memoria: attivazione, abilitazione, ripristino di fabbrica | **Memoria del mouse** |
| DPI | Fino a 5 livelli da 100 a 25600 (passo 50), livello predefinito e sniper, prova dal vivo | Memoria del mouse |
| Polling | 125, 250, 500, 1000 Hz | Memoria del mouse |
| Pulsanti | 11 pulsanti più il livello G-Shift: clic, tasti con Ctrl/Shift/Alt/Win/Alt Gr, tasti multimediali, DPI su/giù/ciclo/sniper, cambio profilo | Memoria del mouse |
| Illuminazione | Logo e indicatore DPI: spento, fisso, respiro, ciclo colori, con anteprima dal vivo | Memoria del mouse |

<table>
  <tr>
    <td><img src="docs/screenshots/keyboard-keys.png" alt="Rimappatura dei tasti"></td>
    <td><img src="docs/screenshots/mouse-dpi.png" alt="DPI e polling del mouse"></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/mouse-buttons.png" alt="Pulsanti del mouse"></td>
    <td><img src="docs/screenshots/mouse-lighting.png" alt="Illuminazione del mouse"></td>
  </tr>
</table>

## Hardware supportato

| Dispositivo | ID USB | Stato |
|---|---|---|
| SteelSeries Apex 3 TKL | `1038:1622` (firmware 3.0.4) | ✅ Supportata, provata su hardware reale |
| Logitech G502 HERO | `046d:c08b` | ✅ Supportato, provato su hardware reale |

PeriHub comunica solo con i dispositivi elencati qui. Non invia mai comandi a dispositivi che non riconosce.

## Hardware in arrivo

L'intenzione è supportare molto più di questi due dispositivi. L'hardware verrà aggiunto un po' alla volta, partendo da quello che usa protocolli che PeriHub conosce già:

- **Mouse Logitech G con HID++ 2.0.** La maggior parte dei mouse Logitech G recenti usa lo stesso protocollo del G502 HERO (DPI, polling, profili in memoria, LED): servono soprattutto gli ID USB e una verifica del formato dei profili.
- **Altre tastiere SteelSeries**, come il resto della serie Apex.
- **Mouse SteelSeries.**
- **Cuffie e altre periferiche**, quando l'hardware espone impostazioni utili.
- **Altri layout e altre lingue** per la rimappatura dei tasti e per l'interfaccia.

Non ci sono scadenze fisse: un dispositivo viene aggiunto quando il suo protocollo si può verificare su hardware reale. Se hai un dispositivo che vorresti vedere supportato, apri una [device request](https://github.com/hex-dump/perihub/issues/new?template=device_request.yml): letture diagnostiche e test sono il modo più rapido per farlo aggiungere.

## Download e installazione

Scarica il file per il tuo sistema dalla [pagina delle release](https://github.com/hex-dump/perihub/releases/latest).

### Windows

1. Scarica `PeriHub_…_x64-setup.exe` (oppure il `.msi`) e avvialo.
2. L'installer non è firmato digitalmente: se SmartScreen lo blocca, clicca **Ulteriori informazioni → Esegui comunque**.
3. **Chiudi SteelSeries GG e Logitech G HUB** e disattivane l'avvio automatico: due programmi che parlano con lo stesso dispositivo si ostacolano.

Non servono driver: PeriHub usa le interfacce HID standard di Windows.

### Linux

**AppImage** (qualsiasi distribuzione, anche Arch e CachyOS):

```sh
chmod +x PeriHub_*_amd64.AppImage
./PeriHub_*_amd64.AppImage
```

**Debian/Ubuntu** (`.deb`) e **Fedora/openSUSE** (`.rpm`): installa il pacchetto con il gestore della tua distribuzione.

#### Permessi USB

I pacchetti `.deb` e `.rpm` installano da soli una regola udev. Con l'AppImage, se PeriHub mostra «permission denied», installala a mano:

```sh
sudo curl -o /etc/udev/rules.d/70-perihub.rules \
  https://raw.githubusercontent.com/hex-dump/perihub/main/packaging/linux/70-perihub.rules
sudo udevadm control --reload && sudo udevadm trigger
```

La regola dà accesso alle sole interfacce HID dei dispositivi supportati, e solo all'utente con la sessione attiva. Se usi già OpenRGB con le sue regole udev, probabilmente non serve.

## Come si usa

- **Keyboard → Lighting**: scegli un effetto. Le modifiche si applicano subito e vengono ricordate.
- **Keyboard → Keys**: scegli il layout (ANSI US, ISO UK, ISO Italian), clicca un tasto, scegli la nuova funzione e premi **Save to keyboard**. Con **With SteelSeries key** modifichi cosa fa ogni tasto mentre tieni premuto il tasto SteelSeries.
- **Mouse**: scegli il profilo in alto e modifica DPI, pulsanti e luci. Nel mouse non viene scritto nulla finché non premi **Save to mouse**. Prima del salvataggio i DPI e i LED si provano dal vivo; uscendo dalla pagina il mouse torna alle impostazioni salvate.
- **G-Shift**: assegna «G-Shift» a un pulsante. Mentre lo tieni premuto, gli altri pulsanti usano le funzioni della scheda G-Shift.

### PeriHub in background

Chiudere la finestra **non** chiude PeriHub. Resta attivo per animare le luci della tastiera e per riapplicarle quando la ricolleghi o il PC si risveglia. Per riaprirlo usa l'icona nell'area di notifica oppure avvialo di nuovo. Per chiuderlo davvero: **Settings → Quit PeriHub**.

L'**avvio automatico** si attiva in Settings. Su Hyprland le voci di avvio automatico XDG funzionano solo se è attivo `xdg-desktop-autostart.target` (uwsm o systemd). In alternativa aggiungi alla configurazione:

```
exec-once = /percorso/di/perihub --hidden
```

Per vedere l'icona nell'area di notifica su Hyprland o Sway serve una barra con il modulo tray, come Waybar o la barra di Noctalia.

## Sicurezza

PeriHub non modifica il firmware. Usa gli stessi comandi di configurazione dei programmi ufficiali, con alcune protezioni in più:

- **Comandi limitati.** Il codice può inviare solo i comandi verificati.
  - Il comando che porta la Apex nel bootloader non è nemmeno rappresentabile.
  - Sul G502 si usano solo le funzioni HID++ `0x0000`, `0x0001`, `0x0003`, `0x0005`, `0x2201`, `0x8060`, `0x8070` e `0x8100`.
- **Scritture verificate.**
  - Mouse: i profili si modificano sopra i byte originali del settore, così i campi non gestiti restano intatti. Ogni settore scritto viene riletto e confrontato byte per byte.
  - Tastiera: la mappa dei tasti si scrive prima in RAM e viene salvata solo se la rilettura corrisponde, altrimenti viene ripristinata.
- **Profili di fabbrica intoccabili.** I settori ROM del mouse non vengono mai scritti e servono per il ripristino.
- **Copie di sicurezza.** Prima di ogni scrittura nella memoria di un dispositivo, PeriHub salva lo stato precedente in
  - Linux: `~/.config/perihub/backups/`
  - Windows: `%APPDATA%\perihub\backups\`

## Domande frequenti

**Posso tenere aperti anche GG o G HUB?**
No, chiudili. Su Windows G HUB, in particolare, prende il controllo del mouse e annullerebbe le modifiche.

**Perdo la garanzia?**
PeriHub non tocca il firmware. Scrive solo le impostazioni (profili, mappa dei tasti) che scrivono anche i programmi ufficiali.

**Perché la tastiera torna all'arcobaleno quando PeriHub è chiuso?**
La Apex 3 TKL non memorizza i colori: anche con GG funziona così. PeriHub li riapplica appena rileva la tastiera. «Built-in rainbow» è l'unico effetto generato dalla tastiera stessa.

**Come torno alle impostazioni di fabbrica?**
Tastiera: **Keys → Reset all to factory**. Mouse: **Factory settings** su ogni profilo. In più ci sono le copie di sicurezza.

**L'interfaccia sarà disponibile in italiano?**
Per ora è in inglese. Altre lingue sono nei piani.

## Limiti noti

- **Windows**: compilato e impacchettato dalla CI, e i test girano su Windows, ma l'app non è ancora stata provata su un PC Windows reale. Le segnalazioni sono benvenute.
- **Etichette da confermare**: le funzioni di fabbrica di SteelSeries + F9/F10, e se il tasto in basso a destra è Menu o Win destro.
- **Non ancora presenti**:
  - cambio profilo automatico in base al gioco o al programma attivo;
  - macro software;
  - illuminazione reattiva ai tasti;
  - compatibilità con GameSense.
- **Fuori dal progetto**: Sonar e Moments di SteelSeries, gli script Lua di G HUB.

## Compilare dai sorgenti

Servono [Rust](https://rustup.rs) stabile (≥ 1.88) e [Node.js](https://nodejs.org) ≥ 20.19. Su Linux servono anche le librerie di sviluppo di WebKitGTK 4.1, libayatana-appindicator, librsvg e libudev:

```sh
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libudev-dev
# Arch/CachyOS
sudo pacman -S webkit2gtk-4.1 libayatana-appindicator librsvg
```

```sh
git clone https://github.com/hex-dump/perihub.git
cd perihub/app
npm install
npm run tauri dev                     # sviluppo, con ricarica automatica
npm run tauri build -- --no-bundle    # eseguibile autonomo in target/release/perihub
npm run tauri build                   # installer per il sistema corrente
```

Non compilare l'app con il solo `cargo build`: quell'eseguibile carica l'interfaccia dal server di sviluppo (`localhost:1420`).

Test e controlli:

```sh
cargo test --workspace          # include test sui settori letti da un G502 HERO reale
cargo clippy --workspace --all-targets
cd app && npm run check         # controllo dei tipi dell'interfaccia
```

### Strumento a riga di comando

Utile per diagnostica e script. Non usarlo mentre l'app è aperta: si contenderebbero gli stessi dispositivi.

```sh
cargo run -p perihub-cli -- status
cargo run -p perihub-cli -- mouse state
cargo run -p perihub-cli -- keyboard keymap
cargo run -p perihub-cli -- keyboard light '{"type":"wave","period_ms":3000,"reverse":false}' 10
```

## Come è fatto

```
crates/perihub-core     libreria Rust: protocolli, effetti, rilevamento, configurazione
  src/apex/             SteelSeries Apex 3 TKL: RGB (report di output) e mappa tasti (feature report)
  src/hidpp/            Logitech HID++ 2.0 e formato dei profili in memoria (CRC-CCITT)
  src/effects.rs        effetti di luce calcolati dal PC
  src/manager.rs        collegamento e scollegamento dei dispositivi
  src/worker.rs         un thread per dispositivo: le richieste non si accavallano mai
  tests/fixtures/       settori reali del G502 HERO usati dai test
crates/perihub-cli      strumento a riga di comando
app/                    interfaccia: Tauri 2 + Svelte 5
packaging/linux         regola udev
```

- La comunicazione con i dispositivi passa da [hidapi](https://github.com/libusb/hidapi). Su Linux il backend è hidraw, quindi tastiera e mouse continuano a funzionare normalmente mentre PeriHub è aperto.
- Su Windows le collezioni HID++ corta e lunga del mouse sono dispositivi separati: PeriHub usa quella lunga (report `0x11`).
- Ogni famiglia di dispositivi è un modulo separato di `perihub-core`: è lì che verrà aggiunto il nuovo hardware.

## Contribuire

Segnalazioni di bug, richieste di dispositivi e pull request sono benvenute.

- **Bug**: indica il sistema operativo, il dispositivo e cosa ha mostrato l'app.
- **Nuovi dispositivi**: apri una [device request](https://github.com/hex-dump/perihub/issues/new?template=device_request.yml) con il modello e l'ID USB.
- **Codice**: prima di aprire una pull request lancia `cargo test`, `cargo clippy` e `npm run check`.

## Sostieni il progetto

PeriHub è gratuito e open source. Se ti ha evitato di installare G HUB o SteelSeries GG e vuoi dire grazie, puoi [offrirmi un caffè](https://buymeacoffee.com/hexdump1).

Le donazioni sono del tutto facoltative e non sbloccano nulla. Sostengono lo sviluppo di PeriHub, compreso l'acquisto di nuovi dispositivi da supportare. Puoi aiutare altrettanto senza spendere: prova PeriHub sul tuo hardware, segnala i bug o apri una device request con i dump in sola lettura.

## Riconoscimenti e licenza

I protocolli sono stati ricostruiti grazie al lavoro di:

- [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB): documentazione del protocollo di illuminazione Apex a 8 zone. Il codice non è stato copiato.
- [libratbag](https://github.com/libratbag/libratbag): Logitech HID++ 2.0 e profili in memoria (MIT).
- [gorkemgun/apex-3-tkl](https://github.com/gorkemgun/apex-3-tkl): mappa dei tasti della Apex 3 TKL (MIT).

PeriHub è distribuito con [licenza MIT](LICENSE). Non è affiliato a SteelSeries né a Logitech; i nomi dei prodotti appartengono ai rispettivi proprietari.
