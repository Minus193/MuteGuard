# MuteGuard - documentazione completa

## Informazioni generali

| Voce | Dettaglio |
| --- | --- |
| Nome | MuteGuard |
| Versione documentata | 1.5.1 |
| Piattaforma | Windows 10 e Windows 11 |
| Architettura distribuita | x64 / AMD64 |
| Tipologia | Utility locale per controllare lo stato mute dei microfoni |
| Interfaccia | Tray, overlay nativo e finestra Settings |
| Licenza | Apache License 2.0 |
| Repository | <https://github.com/Minus193/MuteGuard> |

MuteGuard è un'applicazione Windows leggera che permette di mutare e smutare uno o più microfoni tramite scorciatoie globali, menu del tray e controlli nativi. Lo stato mostrato nel tray e nell'overlay viene letto direttamente da Windows Core Audio.

L'applicazione funziona localmente e non contiene telemetria. Se il controllo aggiornamenti è attivo, effettua al massimo una richiesta anonima al giorno verso la GitHub Releases API pubblica. Un controllo manuale parte solo premendo `Check now`; l'installer viene aperto nel browser solo dopo un clic esplicito. L'installer può inoltre scaricare facoltativamente il runtime WebView2 quando non è già disponibile nel sistema.

## Componenti dell'applicazione

MuteGuard separa il processo in background dalla finestra delle impostazioni:

1. Il processo principale gestisce Core Audio, hotkey, tray, notifiche e overlay.
2. La finestra Settings viene aperta in un processo separato basato su Dioxus e WebView2.
3. La chiusura di Settings libera le relative risorse WebView2 ma lascia attivo il processo in background.
4. Il comando `Exit` nel menu del tray arresta completamente MuteGuard.

Questa separazione consente al processo che rimane nel tray di essere più leggero e di non dipendere da WebView2 per hotkey, controlli audio e overlay.

## Autorità dello stato del microfono

Windows Core Audio è l'unica autorità dello stato mute. Quando viene richiesto un cambio:

```text
azione dell'utente
  -> lettura dello stato corrente
  -> applicazione del nuovo stato
  -> callback Core Audio
  -> nuova lettura dello stato reale
  -> aggiornamento di tray e overlay
```

MuteGuard non presume che una richiesta sia riuscita. Tray e overlay vengono aggiornati dopo la conferma ricevuta dal sottosistema audio.

Quando Windows cambia dispositivo predefinito, MuteGuard rinnova il collegamento alla callback Core Audio. Gli eventi ravvicinati vengono riconciliati dopo 350 ms, mentre un endpoint non ancora pronto viene ritentato ogni secondo fino al ripristino.

## Dispositivi controllabili

Ogni hotkey può controllare uno dei seguenti microfoni:

- `Default microphone`: usa il microfono predefinito per le comunicazioni; se non disponibile, il runtime può utilizzare il dispositivo di cattura predefinito della console.
- Dispositivo specifico: usa direttamente un endpoint di cattura selezionato tramite il relativo ID stabile di Windows.
- `All microphones`: determina la nuova direzione mute dal microfono predefinito per le comunicazioni e applica lo stesso stato a tutti gli endpoint di cattura attivi.

Un dispositivo specifico temporaneamente scollegato rimane salvato e viene mostrato come non disponibile. La selezione torna operativa quando lo stesso endpoint viene ricollegato.

Ogni hotkey salva anche la propria azione. `Toggle mute` legge lo stato reale e richiede quello opposto; `Mute` e `Unmute` impostano uno stato esplicito e sono idempotenti. Se il microfono selezionato è già nello stato richiesto, MuteGuard evita una chiamata `SetMute` inutile. Le configurazioni precedenti prive del campo azione usano automaticamente `Toggle mute`.

## Finestra Settings

La finestra si apre centrata con queste dimensioni:

| Proprietà | Valore |
| --- | ---: |
| Dimensione iniziale | 1200 × 740 px logici |
| Dimensione minima | 760 × 590 px logici |
| Ridimensionabile | Sì |
| Decorazione | Barra del titolo personalizzata |

Le sezioni operative usano un layout responsive a mosaico. Il numero di colonne dipende dallo spazio disponibile e le card hanno altezza indipendente. La Guide usa invece una singola colonna larga per facilitare la lettura.

Le modifiche valide vengono salvate immediatamente. Non esiste un pulsante Apply. Un valore incompleto o non valido viene rifiutato senza sostituire l'ultimo valore corretto.

## General

### Start with Windows

Registra MuteGuard nella chiave utente:

```text
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
```

La voce si chiama `MuteGuard`. A ogni avvio reale del processo in background, l'applicazione confronta una sola volta la registrazione attuale con il percorso dell'eseguibile:

- non scrive nulla se la voce è già corretta;
- ripara una voce mancante o obsoleta quando l'opzione è attiva;
- elimina la propria voce quando l'opzione è disattivata;
- non esegue polling periodico del registro.

### Mute microphone on startup

Quando attiva, applica mute una sola volta dopo che il microfono predefinito per le comunicazioni diventa disponibile. Se l'endpoint non è ancora pronto, il tentativo viene ripetuto ogni 2 secondi. Dopo l'applicazione non continua a forzare lo stato.

### Microphone change notifications

Mostra una notifica Windows quando il microfono predefinito per le comunicazioni:

- cambia;
- viene scollegato;
- torna disponibile;
- richiede il rinnovo del monitoraggio Core Audio.

Le notifiche usano l'identità applicativa `local.muteguard.notifications.v1` e l'immagine PNG ufficiale di MuteGuard. Facendo clic sulla notifica si apre Settings. Se il canale moderno non è disponibile, il runtime può usare la notifica del tray come fallback.

### Check for updates

Quando attivo, interroga la release stabile più recente del repository pubblico al massimo una volta ogni 24 ore. La richiesta usa WinHTTP e non invia credenziali, token o dati di configurazione.

`Check now` ignora l'intervallo giornaliero. Se esiste una versione più recente, Settings mostra `Download update` e la notifica Windows apre l'installer x64 ufficiale della release. MuteGuard non scarica né installa aggiornamenti in modo silenzioso.

Lo stato dell'ultimo controllo, inclusi orario, versione rilevata ed eventuale errore, viene salvato separatamente dalla configurazione per evitare conflitti con modifiche contemporanee ai Settings.

### Application accent

Sono disponibili due sorgenti:

- `System color`: segue il colore di accento corrente di Windows.
- `Custom color`: utilizza un colore indipendente scelto nell'editor MuteGuard.

Il colore personalizzato accetta preset, selezione HSV, valori RGB e un codice HEX esatto. Le modifiche vengono applicate a controlli, focus ring ed elementi evidenziati.

## Hotkeys

MuteGuard supporta più hotkey globali contemporaneamente.

### Tipi di combinazione

Sono accettati:

- singoli tasti;
- soli modificatori;
- combinazioni tradizionali con Ctrl, Alt, Shift o Windows;
- accordi simultanei arbitrari, per esempio `A+B`;
- tasti del mouse supportati dal sistema di cattura.

Non è obbligatorio includere Ctrl. La registrazione termina al primo rilascio di un elemento della combinazione, conservando l'intero accordo rilevato.

### Operazioni disponibili

- `Add hotkey`: crea una nuova voce e avvia immediatamente la registrazione.
- `Record`: sostituisce la combinazione della voce selezionata.
- `Cancel`: termina la registrazione mantenendo il valore precedente.
- `Action`: sceglie `Toggle mute`, `Mute` o `Unmute`. Le azioni esplicite non richiedono modifiche a Core Audio quando lo stato è già quello richiesto.
- `Microphone`: sceglie il microfono predefinito, un endpoint specifico o tutti i microfoni.
- `Ignore modifiers`: permette l'attivazione anche in presenza di modificatori aggiuntivi non registrati nella combinazione.
- `Delete`: rimuove soltanto la hotkey selezionata.

Durante la registrazione, un controllo temporaneo dello stato fisico dei tasti supporta gli hook di Windows per evitare che un evento perso lasci la scheda bloccata.

## Overlay

L'overlay è un indicatore nativo click-through. Non prende il focus e non intercetta il mouse.

### Attivazione e visibilità

| Opzione | Comportamento |
| --- | --- |
| `Always visible` | Rimane visibile indipendentemente dallo stato mute |
| `Visible when muted` | È visibile soltanto quando il microfono è mutato |
| `Visible when unmuted` | È visibile soltanto quando il microfono è attivo |
| `Temporarily after a change` | Compare dopo una transizione mute o unmute |

Per la modalità temporanea, la durata è regolabile da 0,5 a 10 secondi con incrementi di 0,5 secondi.

### Monitor

È possibile selezionare uno o più display. MuteGuard crea un overlay sincronizzato per ogni monitor disponibile scelto dall'utente.

- Il monitor primario viene riconosciuto automaticamente.
- Le selezioni duplicate vengono eliminate.
- I display scollegati rimangono nella configurazione e tornano operativi dopo la riconnessione.
- Se nessun monitor selezionato è disponibile, viene usato un solo fallback sul display primario.
- Un evento `WM_DISPLAYCHANGE` ricostruisce immediatamente l'insieme degli overlay.

### Posizione

Sono disponibili nove ancoraggi:

- alto sinistra, alto centro, alto destra;
- centro sinistra, centro, centro destra;
- basso sinistra, basso centro, basso destra.

La posizione predefinita è alto centro. Il runtime usa l'area di lavoro del monitor e mantiene un margine di sicurezza di 10 px, evitando taskbar e bordi disponibili dello schermo.

La casella `Preview` forza temporaneamente la visualizzazione dell'overlay. Non viene salvata e viene disattivata lasciando la sezione o chiudendo Settings. Un lease rinnovabile di 2,5 secondi impedisce che una preview abbandonata rimanga visibile dopo una chiusura anomala.

### Stile del contenuto

| Stile | Contenuto |
| --- | --- |
| `Microphone icon` | Solo icona del microfono |
| `Icon and text` | Icona e testo di stato |
| `Text only` | Solo testo di stato |
| `Status dot` | Indicatore compatto a punto |

I controlli non applicabili allo stile selezionato vengono nascosti.

### Libreria delle icone

Sono disponibili 17 famiglie con variante mutata e non mutata: MDI, Fluent,
Lucide, Phosphor, Solar, Hugeicons, Tabler, Material, Remix, IconMoon, Gravity,
Eva, UIcons, Basil, Pepicons, MingCute e Ming Fill.

L'icona predefinita è MDI. La forma mutata o non mutata segue lo stato reale e il contesto di visibilità dell'overlay.

### Colore dell'icona

- `Colored`: usa il colore personalizzato dell'overlay.
- `Monochrome`: usa un colore neutro adatto al contenuto.
- `System color`: segue l'accento di Windows.

I preset condivisi sono:

```text
#FFFFFF  #BDC3C8  #222F3D  #7E40FD
#2980B9  #F39C19  #2ECC70  #E84B3C
```

Il colore personalizzato supporta editor HSV, feedback RGB e codice HEX esatto.

### Testo e dimensioni

| Controllo | Intervallo o comportamento |
| --- | --- |
| Muted label | Testo mostrato nello stato mutato |
| Unmuted label | Testo mostrato nello stato attivo |
| Font | Famiglia installata in Windows |
| Font weight | Da 100 a 900, incrementi di 100 |
| Scale | Da 10% a 400%, incrementi di 5% |
| Content opacity | Da 20% a 100%, incrementi di 5% |

I font salvati rimangono in configurazione anche quando temporaneamente non disponibili. Le varianti Segoe UI con peso incorporato vengono normalizzate alla famiglia base, lasciando al controllo `Font weight` la gestione dello spessore.

### Sfondo

| Controllo | Valori |
| --- | --- |
| Background style | Dark, Light, Transparent |
| Background opacity | Da 0% a 100%, incrementi di 5% |
| Corner radius | Da 0 a 24 px |
| Show border | Attiva o disattiva il bordo antialias |
| Border color | Colore HEX, preset o editor completo |

Lo sfondo Dark usa `#131313`. Contenuto e sfondo hanno opacità indipendenti. La superficie nativa viene composta in ARGB trasparente e usa antialiasing dedicato per bordi, angoli, testo e glifi.

## Tray

### Stili disponibili

- `Logo`: mostra sempre il logo MuteGuard.
- `Mic status`: usa una delle famiglie di icone microfono e segue lo stato mute.
- `Color dot`: usa un indicatore di stato compatto.

Per `Mic status` sono disponibili le stesse 17 famiglie dell'overlay. Il colore può essere `Colored`, `Monochrome` o `System color`. Il colore personalizzato del tray è indipendente da quello dell'overlay.

### Azioni del mouse

- Click sinistro: apre o porta in primo piano Settings.
- Click destro: apre il menu contestuale.

Il menu contiene:

- `Mute microphone` oppure `Unmute microphone`, in base allo stato attuale;
- `Settings`;
- `Exit`.

Quando Core Audio non è disponibile, il comando mute viene sostituito da una voce disabilitata che indica il microfono non disponibile.

Se Explorer ricrea l'area di notifica, MuteGuard tenta di ripristinare l'icona del tray ogni 2 secondi finché l'operazione riesce.

## Sound

Il feedback sonoro è facoltativo e indipendente dal volume del microfono Windows.

### Configurazione

- Attivazione generale tramite `Sound feedback`.
- Volume da 0% a 100%, incrementi di 1%.
- Sorgente mute e unmute configurabile separatamente.
- `Built-in tone` per il segnale incluso nell'applicazione.
- `Custom WAV` per un file personalizzato.

### Requisiti dei file personalizzati

Il file deve essere:

- WAV PCM non compresso;
- 16 bit;
- lungo al massimo 5 secondi;
- grande al massimo 12 MB;
- coerente nei chunk RIFF e nei metadati PCM.

Il salvataggio sostituisce atomicamente il file personalizzato precedente dello stesso stato. Se la sorgente Custom non è valida o disponibile, viene riprodotto il tono integrato.

I pulsanti `Preview mute` e `Preview unmute` usano lo stesso percorso audio degli eventi reali ma non cambiano lo stato del microfono. Le richieste ravvicinate possono sovrapporsi senza interrompersi o attendersi; sono consentite al massimo 16 riproduzioni concorrenti.

## Diagnostics

La sezione è locale, read-only e non modifica la configurazione.

### Application

- versione;
- architettura;
- stato del processo in background;
- stato di caricamento della configurazione;
- stato di Start with Windows.

### Windows

- build del sistema operativo;
- disponibilità del runtime WebView2;
- disponibilità delle API Mica.

### Audio

- disponibilità di Core Audio;
- stato mute corrente;
- ID dell'endpoint predefinito;
- stato delle notifiche sui dispositivi;
- stato del feedback sonoro;
- disponibilità dei file personalizzati mute e unmute.

### Input and overlay

- numero di hotkey configurate;
- stato dell'overlay;
- display configurati;
- display attualmente rilevati.

### Updates

- stato dei controlli automatici;
- controllo attualmente in corso;
- ultima release rilevata;
- ultimo controllo riuscito;
- ultimo errore di rete o API.

`Copy diagnostics` copia negli appunti un report contenente i valori diagnostici visibili. Credenziali e percorsi personali completi vengono esclusi.

## Guide integrata

Il pulsante con il punto di domanda nella parte inferiore della sidebar apre la guida interna. La guida:

- copre General, Hotkeys, Overlay, Tray, Sound e Diagnostics;
- mostra un argomento alla volta;
- descrive ogni controllo e le relative condizioni;
- usa righe allineate tra nome e descrizione;
- ricorda che i valori validi vengono salvati immediatamente.

## Configurazione e dati locali

### Percorsi

| Contenuto | Percorso |
| --- | --- |
| Configurazione | `%APPDATA%\MuteGuard\config.json` |
| Cache aggiornamenti | `%APPDATA%\MuteGuard\update-cache.json` |
| Suono mute personalizzato | `%APPDATA%\MuteGuard\sounds\mute.wav` |
| Suono unmute personalizzato | `%APPDATA%\MuteGuard\sounds\unmute.wav` |
| Dati WebView2 | `%LOCALAPPDATA%\muteguard` |
| Installazione predefinita | `%LOCALAPPDATA%\Programs\MuteGuard` |

La configurazione viene normalizzata prima dell'uso e scritta atomicamente tramite file temporanei univoci e tentativi limitati in caso di lock transitorio. Un file non valido può essere preservato come backup con nome univoco prima del ripristino dei valori predefiniti.

Il file di configurazione contiene:

- hotkey, azioni e rispettivi microfoni;
- preferenze di avvio;
- aspetto di Settings;
- configurazione completa dell'overlay;
- stile del tray;
- notifiche dei dispositivi;
- preferenza per i controlli automatici degli aggiornamenti;
- configurazione del feedback sonoro;
- opzioni avanzate supportate dal sistema.

## Valori predefiniti

| Impostazione | Valore predefinito |
| --- | --- |
| Start with Windows | Disattivato |
| Mute microphone on startup | Disattivato |
| Microphone change notifications | Attivato |
| Check for updates | Attivato |
| Application accent | System color |
| Colore accent custom | `#7D42FB` |
| Overlay | Attivato |
| Visibilità overlay | Visible when muted |
| Monitor overlay | Display primario |
| Posizione overlay | Alto centro |
| Durata temporanea | 2 secondi |
| Stile contenuto | Microphone icon |
| Famiglia icona | MDI |
| Colore icona | Colored, `#7C83FF` |
| Scale | 100% |
| Content opacity | 100% |
| Muted label | `Microphone muted` |
| Unmuted label | `Microphone on` |
| Font | Segoe UI |
| Font weight | 700 |
| Background style | Dark |
| Background opacity | 90% |
| Corner radius | 6 px |
| Border | Attivo, `#323441` |
| Tray icon style | Mic status |
| Tray icon color | Colored, `#7C83FF` |
| Sound feedback | Disattivato |
| Sound volume | 45% |
| Mute / unmute source | Built-in tone |

## Installazione

L'installer NSIS è per utente e non richiede l'installazione in una cartella di sistema. Durante l'installazione:

1. arresta in modo controllato un'eventuale istanza già attiva;
2. copia i file applicativi escludendo i file Markdown;
3. crea collegamenti nel menu Start e sul desktop;
4. registra l'identità usata dalle notifiche Windows;
5. registra il protocollo locale `muteguard://`;
6. registra MuteGuard nell'elenco delle applicazioni installate;
7. verifica la presenza di WebView2;
8. propone il bootstrapper ufficiale Microsoft se WebView2 manca.

L'assenza di WebView2 impedisce l'apertura di Settings, ma non blocca hotkey, tray, overlay e controllo del microfono.

## Disinstallazione

L'uninstaller esegue una rimozione completa:

1. arresta il processo in background e Settings;
2. elimina `%LOCALAPPDATA%\muteguard`;
3. elimina `%APPDATA%\MuteGuard`, inclusi configurazione, backup e suoni custom;
4. rimuove la voce Start with Windows;
5. rimuove identità di notifica e protocollo applicativo;
6. elimina collegamenti, file installati e registrazione dell'uninstaller.

Se una cartella è temporaneamente bloccata, la rimozione viene ritentata e può essere pianificata al riavvio.

## Versione portable

Il pacchetto portable contiene:

- `muteguard.exe`;
- `WebView2Loader.dll`;
- asset Dioxus compilati;
- `muteguard.ico` e `muteguard.png`;
- licenza.

Il pacchetto non contiene file Markdown. Configurazione, suoni e dati WebView2 rimangono nei percorsi utente indicati sopra, anche quando l'eseguibile viene avviato dalla cartella portable.

## Comandi interni

| Comando | Scopo |
| --- | --- |
| `muteguard.exe` | Avvia o porta avanti il processo in background |
| `muteguard.exe --settings` | Apre o porta in primo piano Settings |
| `muteguard.exe --toggle-mute` | Richiede il toggle del microfono predefinito |
| `muteguard.exe --mute` | Muta esplicitamente il microfono predefinito |
| `muteguard.exe --unmute` | Smuta esplicitamente il microfono predefinito |
| `muteguard.exe --exit-all` | Arresta processo in background e Settings |
| `muteguard://settings` | Apre Settings dalle notifiche Windows |
| `muteguard://toggle-mute` | Richiede il toggle tramite protocollo applicativo |
| `muteguard://mute` | Richiede mute tramite protocollo applicativo |
| `muteguard://unmute` | Richiede unmute tramite protocollo applicativo |

Questi comandi sono principalmente destinati all'integrazione interna, all'installer e alle notifiche.

## Modello prestazionale

Il runtime è guidato dagli eventi. Non esegue polling continuo di:

- configurazione;
- dispositivi audio;
- sessioni o processi;
- utilizzo del microfono;
- inattività dell'utente;
- rete, tranne il singolo controllo aggiornamenti giornaliero quando attivo.

I timer sono limitati a operazioni finite o attivate da eventi, tra cui:

- durata dell'overlay temporaneo;
- lease della preview;
- retry del mute all'avvio;
- riconciliazione delle hotkey al rilascio;
- debounce e riconnessione Core Audio;
- ripristino del tray dopo un riavvio di Explorer.

## Privacy e sicurezza

- Nessuna telemetria.
- Controllo aggiornamenti anonimo e disattivabile, al massimo una volta al giorno.
- Nessuna autenticazione o credenziale memorizzata dall'applicazione.
- Nessuna telemetria o trasmissione della configurazione.
- Report diagnostico privo di credenziali e percorsi personali completi.
- Configurazione e suoni restano sul computer dell'utente.
- Eseguibile x64 con ASLR, DEP/NX e high-entropy virtual addressing abilitati.
- Nessuna sezione PE è contemporaneamente scrivibile ed eseguibile.

Gli eseguibili distribuiti non sono firmati digitalmente. Windows SmartScreen o software antivirus basati sulla reputazione possono quindi mostrare un avviso per un hash appena generato.

## Tecnologie

- Rust 1.98, edition 2024;
- Dioxus 0.7 per Settings;
- WebView2 per il rendering della finestra Settings;
- Windows Core Audio per stato e controllo mute;
- API Win32 per tray, hotkey, monitor, registro e notifiche;
- WinHTTP per il controllo anonimo della release pubblica;
- renderer nativo per l'overlay;
- WinMM waveOut per il feedback sonoro sovrapponibile;
- resvg per le icone SVG;
- NSIS per l'installer;
- Docker e target `x86_64-pc-windows-gnu` per la pipeline riproducibile x64.

## Build e controlli

La pipeline completa è:

```powershell
.\build-docker.ps1
```

La build usa il container `muteguard-builder:rust-1.98-dx-0.7.6` senza accesso di rete e produce sotto `dist\<version>`:

- cartella portable;
- ZIP portable;
- installer EXE;
- ZIP dell'installer.

I controlli verificati includono:

```text
cargo fmt --all -- --check
cargo clippy --offline --locked --tests --target x86_64-pc-windows-gnu -- -D warnings
cargo test --offline --locked --no-run --target x86_64-pc-windows-gnu
dx build --desktop --release --target x86_64-pc-windows-gnu --frozen
makensis
```

Il generatore dell'icona produce frame PNG da 16, 20, 24, 32, 40, 48, 64 e 256 px. Il frame 256 viene derivato dal render canonico da 1024 px e viene rifiutato se il foreground risulta mancante o tagliato.

## Limiti intenzionali

MuteGuard è focalizzato sul controllo mute. Non è un mixer audio completo e non include:

- regolazione del gain del microfono;
- cambio automatico dei dispositivi di ingresso o uscita;
- rilevamento per-processo dell'utilizzo del microfono;
- azioni hold-to-mute o hold-to-talk;
- automazioni basate sull'inattività;
- supporto per controller;
- installazione silenziosa o forzata degli aggiornamenti.

## File di riferimento

- [README.md](README.md): panoramica breve e istruzioni di build.
- [VERIFICATION.md](VERIFICATION.md): controlli tecnici e hash degli artefatti correnti.
- [RELEASE_NOTES.md](RELEASE_NOTES.md): note della release corrente.
- [LICENSE](LICENSE): testo della licenza.
- [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md): attribuzioni delle dipendenze e degli asset.
