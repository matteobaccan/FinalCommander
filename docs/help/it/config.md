# Dove sta ogni cosa

Tutto è un file che puoi leggere, confrontare e copiare su un'altra macchina:

    fcmd.toml          le impostazioni — e i tuoi tasti, temi, server, filtri e colori
    fcmd-state.toml    quello che il programma ricorda: dov'erano i pannelli, le cronologie
    fcmd-macros.toml   le macro che hai registrato
    fcmd-menu.toml     il tuo menu di F2
    fcmd-secrets.vault chiavi e password, cifrate con una password tua
    fcmd-plugins/      i plugin
    fcmd-jobs/         i giornali delle copie in corso — c'è solo finché ce n'è una

Stanno uno accanto all'altro nella cartella dove la piattaforma tiene le impostazioni —
`~/.config` su Linux, `~/Library/Application Support` su macOS, `%APPDATA%` su Windows —
oppure accanto all'eseguibile, quando lì c'è un `fcmd.toml`, che è come una copia su una
chiavetta USB si porta dietro tutto. `fcmd --paths` dice esattamente dove sta ciascuno su
questa macchina, e `fcmd --dump-config` stampa le impostazioni con cui il programma sta davvero
girando, che è il modo più rapido di vedere come si chiama una chiave.

L'orologio nell'angolo in alto a destra è spento finché non lo chiedi, in `F9 → Opzioni →
Impostazioni dell'interfaccia`: è l'unica cosa sullo schermo che cambia senza che nessuno prema
niente, quindi con lui acceso il programma si sveglia quando scatta il minuto per ridisegnare
quattro caratteri. È un ridisegno al minuto, e su `ssh` è traffico che stai pagando tu — che è
tutto il motivo per cui non è acceso di default.

`F9 → Opzioni → Tutti i parametri` è ogni impostazione che esiste, in un elenco: la chiave, il
valore a cui sta, e un `·` su quelle che non sono più quelle con cui il programma è stato
costruito. Scrivi per filtrare, `Invio` o `F4` per scrivere un valore, `Spazio` per invertire un
sì-o-no, `Canc` per rimettere il valore di partenza. I dialoghi sopra, in quel menu, sono la
dozzina di impostazioni che si cambiano più spesso, con le parole che dicono cosa significa
ognuna; questo è tutto il resto, ed è il modo di scoprire come si chiama una chiave.

`Opzioni → Salva impostazioni` scrive il file delle impostazioni; nient'altro lo fa, quindi
un'impostazione cambiata per vedere cosa fa non c'è più al prossimo avvio. Se preferisci il
contrario, *Salva le impostazioni all'uscita* in *Opzioni → Impostazioni dell'interfaccia*
(`ui.save_setup_on_exit`) scrive il file anche quando esci. Dov'erano i pannelli è un altro
file e torna in ogni caso.

# Chiedere a un'AI

Il modo più semplice è la riga **AI** di `Alt+F1` / `Alt+F2`: trasforma il pannello nella lista
dei tuoi servizi. `F7` ne aggiunge uno, `F4` lo modifica, `F8` lo toglie. `Invio` fa il login —
fcmd chiede al servizio l'elenco dei suoi modelli, che prova indirizzo e chiave senza chiedere
niente al modello, e la riga di stato dice cosa è tornato (`HTTP 200 in 120 ms, 12 modelli`), o
perché non è andata (un server che non è acceso, una chiave sbagliata). Da lì il pannello è la
**conversazione**, mostrata lì dentro man mano, in Markdown: in cima i file del contesto, ogni
domanda a destra, ogni risposta a sinistra mentre il modello la scrive e, finché sta ancora
arrivando, una rotellina che gira con i secondi di attesa. La cornice dice il modello che ha
risposto, quanto è piena la sua context window (`ctx 1.5k/32.8k (4%)`; la dimensione compare
quando il server la dichiara al login) e quanto prenderà, circa, la prossima domanda
(`prossima ≈ 2.1k`: la conversazione fin qui, che viene rimandata, i file e la domanda, a
quattro byte per token). La risposta resta nel pannello; non apre un viewer. Le frecce e `PagSu`/`PagGiù` la
fanno scorrere; `Ctrl+R` la ricostruisce e la lascia una conversazione:

* quello che scrivi sulla riga di comando sotto di lui va all'AI, come seguito della stessa
  conversazione — senza prefissi;
* `F5` nell'altro pannello mette i file selezionati lì nel **contesto**, e partono con ogni
  domanda da quel momento — prima della prima domanda che li porta, un dialogo dice quali file,
  quanto pesano e a quale host vanno;
* `F8` elenca i file del contesto con la loro dimensione: `Enter` o `Canc` toglie quello sotto
  il cursore — il file resta sul disco — e l'ultima riga li toglie tutti; senza file nel
  contesto, `F8` comincia una nuova conversazione;
* `F7` elenca le conversazioni tenute finora, dalla più recente, ognuna con la sua prima
  domanda: `Enter` riprende quella, e la prossima domanda la porta con sé;
* `Esc` mentre il modello sta ancora rispondendo interrompe la domanda lì: non se ne vede altro
  e non viene scritto nulla;
* una domanda che prenderebbe più di quanto il modello contiene si ferma prima a un dialogo che
  lo dice, con la stima, anche quando i file sono quelli già mandati.

Il resto del capitolo è la stessa cosa dalla riga di comando.

`ai: domanda` sulla riga di comando chiede a un modello linguistico e apre la risposta nel
visualizzatore. I servizi sono blocchi `[[ai]]` in `fcmd.toml`:

```toml
[[ai]]
name = "claude"
url = "https://api.anthropic.com/v1/messages"
api = "anthropic"            # openai, anthropic oppure ollama
model = "claude-sonnet-5-5"
token_env = "ANTHROPIC_API_KEY"
```

I modelli più recenti di OpenAI — quelli di ragionamento e GPT-5 — vogliono una richiesta
un po' diversa: `max_completion_tokens` al posto di `max_tokens`, a volte il ruolo `developer`
al posto di `system`, e una risposta non in streaming finché l'organizzazione non è
verificata. fcmd manda a OpenAI il nome nuovo fin dall'inizio, e quando un servizio rifiuta una
di queste cose e dice cosa vuole al suo posto, richiede nel modo indicato e se lo ricorda per
la sessione.

Non serve scriverli a mano: *Comandi → Servizi AI* li elenca come le sessioni di NetBox. `Ins`
ne aggiunge uno con il suo dialogo (nome, protocollo, indirizzo, modello, variabile della
chiave) — cambiare protocollo riscrive l'indirizzo per quel protocollo, sullo stesso host, e
un indirizzo scritto senza `http://` o `https://` prende `http` su questa macchina e `https`
altrove; un host da solo prende il percorso solito del protocollo, e una lista di modelli
incollata al posto dell'indirizzo torna all'endpoint accanto; mentre una domanda è in corso,
l'angolo della riga di stato dice quale servizio sta pensando e da quanti secondi — `F4` lo
modifica, `Canc` lo toglie, e `Enter` lo rende quello a cui chiede `ai:`. Se
la variabile della chiave è vuota o non c'è, la chiave viene chiesta la prima volta che serve e
tenuta in `fcmd-secrets.vault`, il file cifrato di fcmd accanto alle impostazioni — così viaggia
con loro su un'altra macchina. La prima volta scegli la password di quel file (vuota: la chiave
resta solo per questa sessione). Dopo, la password si chiede una volta, resta in memoria mentre
lavori, e viene dimenticata dopo cinque minuti senza toccare un tasto; il segreto successivo la
richiede. Il file è AES-256-GCM con una chiave derivata dalla password con PBKDF2-SHA256, e non
contiene mai niente in chiaro.

Il primo blocco è quello a cui chiede `ai:`; `ai: locale: domanda` chiede al blocco chiamato
`locale`. La chiave non è mai scritta in `fcmd.toml`: `token_env` dice in quale variabile
d'ambiente si trova, e se non ce n'è nessuna finisce nel file dei segreti descritto sopra. `api = "openai"` va con ogni server che parla l'API chat di OpenAI, e `ollama` con un
modello che gira sulla tua macchina, che non vuole chiavi.

Si invia solo la domanda, a meno che dei file siano **selezionati**: allora un dialogo dice
quanti, quanto pesano e a quale host vanno, e partono solo se premi *Invia*. Ogni risposta è un
file Markdown nella cartella `fcmd-ai` accanto alle impostazioni, e `ai:` senza niente dopo apre
quella cartella nel pannello, così quello che hai chiesto resta lì da rileggere. La riga di
stato dice quanto è costata la risposta, nei token contati dal servizio, e lo dice anche il file
della risposta.

Altri modi di chiedere:

| Cosa scrivi o premi | Cosa fa |
| --- | --- |
| `ai+ domanda` | fa una **domanda di seguito** nella stessa conversazione: domande e risposte precedenti vengono rimandate (nessun servizio tiene la conversazione), e la nuova risposta si aggiunge allo stesso file. La conversazione è l'ultima risposta di questa sessione, o quella sotto il cursore nella cartella `fcmd-ai` |
| `ai! cosa fare` | chiede **un comando** per la tua shell e il tuo sistema, e lo mette sulla riga di comando — **senza eseguirlo**: leggilo, cambialo, e premi `Invio` tu |
| `Ctrl+Alt+X` nel visualizzatore o nell'editor | chiede **che cos'è questo**: il blocco marcato nell'editor, o quello che c'è sullo schermo, parte con il nome del file — dopo un dialogo che dice quanto e a quale host |
| `Ctrl+Alt+X` nei pannelli, dopo un comando fallito | chiede **perché è fallito**: il comando, il suo codice di uscita e la fine di ciò che ha stampato — al massimo le ultime 200 righe, quelle che mostra `Ctrl+O` — partono dopo lo stesso dialogo, e la risposta si apre nel visualizzatore. Quando un altro comando è finito bene non c'è più niente da chiedere |
| `Ctrl+Alt+U` sui file selezionati | chiede un **modello di rinomina** a parole, e apre con quello il dialogo di rinomina multipla — vedi *Rinominare molti file in una volta* |
| `Ctrl+Alt+Q` nei pannelli o nel dialogo di ricerca | chiede una **ricerca** a parole — *i pdf sopra i 10 MB modificati la settimana scorsa* — e riempie con quella il dialogo di ricerca; non viene cercato niente fino al suo `OK` — vedi *Trovare le cose* |
| `Ctrl+Alt+W` nell'editor | fa **riscrivere** il blocco marcato, mostra il risultato come confronto e lo inserisce solo con `Enter` — vedi *Far riscrivere un blocco* |
| `Shift+F7` in un repository | scrive una **bozza del messaggio di commit** dal diff in staging — dopo lo stesso dialogo — e apre il prompt del commit con quella, da leggere e cambiare prima che si committi qualcosa |

Una domanda al modello non è un comando: `!` e gli altri metasimboli non vengono espansi.

## Cosa viene mascherato prima di partire

I file contengono chiavi: un `.env` accanto ai sorgenti, un token incollato in uno script, una
chiave privata in una cartella selezionata tutta intera. Prima che una domanda lasci la
macchina — la domanda stessa, i file che partono con lei, il blocco dall'editor, i turni
precedenti della conversazione — fcmd maschera ciò che ha l'aspetto di un segreto e scrive
`[REDACTED:tipo]` al suo posto:

| Cosa viene trovato | Cosa viene scritto al suo posto |
| --- | --- |
| un blocco di chiave privata (da `-----BEGIN … PRIVATE KEY-----` alla sua fine) | `[REDACTED:private-key]` |
| le chiavi che dicono cosa sono: AWS (`AKIA…`), GitHub (`ghp_…`, `github_pat_…`), GitLab (`glpat-…`), Slack (`xoxb-…`), Stripe (`sk_live_…`), OpenAI e Anthropic (`sk-…`), Google (`AIza…`), npm (`npm_…`), un JWT | `[REDACTED:aws-key]`, `[REDACTED:github-token]`, `[REDACTED:api-key]` e così via |
| la password in un indirizzo: `postgres://app:segreto@db/shop` | `postgres://app:[REDACTED:url-password]@db/shop` |
| quello che segue `Authorization: Bearer` o `Basic` | `[REDACTED:auth-header]` |
| il valore di una riga `CHIAVE=valore`, `chiave: valore` o `"chiave": "valore"` la cui chiave finisce in `password`, `secret`, `token`, `api_key`, `access_key`, `private_key`, `credential` e simili | `[REDACTED:assignment]` |

Il nome della chiave, l'host e l'utente di un indirizzo restano: sono ciò per cui vale la pena
mandare la riga. Un valore che è un riferimento (`${VAR}`), un numero o una chiamata in un
programma non viene toccato. Nel dubbio maschera, e resta comunque una rete, non una garanzia:
una password che è una parola qualunque in una frase qualunque non ha una forma da trovare.

Quello che ha fatto si vede. Il dialogo che elenca i file guadagna una riga un attimo dopo
essersi aperto — `Segreti mascherati prima dell'invio: 2 (1 assignment, 1 aws-key)` — e lì
**`F3`** apre **che cosa parte**, esattamente come partirà: l'istruzione, la conversazione fin
qui, la domanda con i suoi file, mascherati. `Esc` torna al dialogo, a cui rispondi ancora tu.
In una conversazione, `F3` nella lista `F8` del contesto mostra lo stesso per ciò che parte con
ogni domanda. Dopo la risposta la riga di stato dice quanti segreti sono stati mascherati,
accanto al costo, e lo dice anche la seconda riga del file della risposta.

La mascheratura è **accesa** finché non la spegni: `F5` nella lista dei servizi (la riga AI di
`Alt+F1`) la commuta, e la cornice del pannello dice com'è. In `fcmd.toml`:

```toml
[ai_guard]
redact = true
patterns = ['ACME-\d{6}', 'codice cliente: (\d+)']   # i tuoi, come espressioni regolari
```

Un tuo pattern maschera il suo primo gruppo quando ne ha uno — così le parole prima restano — e
altrimenti tutto ciò che ha trovato; compaiono come `[REDACTED:custom]`.

## Solo modelli su questa macchina

**Solo locale** è un interruttore per quando niente deve uscire: acceso, fcmd chiede solo a un
servizio il cui indirizzo è questa macchina — `localhost` e ciò che gli sta sotto, `127.0.0.1` e
il resto di `127.x.x.x`, `::1`, un socket nel file system (`unix:/run/llm.sock`) — e agli host
che elenchi come tuoi. Ogni altro servizio viene **rifiutato prima che parta qualcosa**: la
domanda, i file, il login che manderebbe la chiave. La riga di stato dice il servizio e l'host
a cui sarebbe andato.

`F6` nella lista dei servizi (la riga AI di `Alt+F1`) lo commuta; la cornice allora dice
`solo locale`, e ogni riga che verrà rifiutata lo dice. In `fcmd.toml`:

```toml
[ai_guard]
local_only = true
local_hosts = ["gpu-box.lan", "192.168.1.20"]   # un modello sulla tua rete conta come locale
```

Un indirizzo viene giudicato per com'è scritto, non risolto: un nome che per caso punta a
`127.0.0.1` non è locale finché non lo elenchi, perché risolverlo è già una domanda fatta a
qualcun altro. Di default è spento.

# Aggiornamenti

`Opzioni → Verifica aggiornamenti` chiede a GitHub qual è l'ultima versione e, se è più nuova
di quella in uso, propone di aprirne la pagina nel browser; l'indirizzo della pagina è anche nel
dialogo, da copiare su una macchina senza desktop. Non scarica e non installa niente: gli
archivi e il changelog sono su quella pagina. Il programma lo chiede solo allora, a meno che
*Cerca una nuova versione una volta al giorno* in *Opzioni → Impostazioni di sistema*
(`general.update_check`) sia attivo; è spento, perché il programma non apre connessioni che non
hai chiesto. Acceso, il controllo gira in background al massimo una volta al giorno, non dice
niente se non c'è niente di nuovo o manca la rete, e propone ogni nuova versione una volta sola.

# Programmi esterni

Quasi tutto fcmd lo fa da sé: zip, la famiglia tar, 7z, `ar` e `.deb`, `cpio` e `rpm` li legge
in processo, SSH, FTP, WebDAV, S3 e IMAP li parla in processo, e il visualizzatore disassembla
x86 e AArch64 per conto suo. Per poche cose invece esegue un programma già presente sulla
macchina, mai collegato dentro di sé. `Alt+F1` → *Programmi esterni* li elenca tutti, con la
spunta se sono nel `PATH` e la croce se non ci sono, la versione che dichiarano e cosa fcmd
chiede loro. `F3` su una riga è la sua scheda: dov'è, a cosa serve e, se manca, come
procurarselo, con il suo sito. `F4` lo installa: mostra la riga che eseguirà — `winget` su
Windows, `brew` su macOS, `apt`, `dnf`, `pacman` o `apk` altrove, quello che c'è su questa
macchina — e, quando dici sì, la esegue in un terminale suo, così `sudo` può chiederti la
password e `winget` l'accettazione dei suoi termini. `Ctrl+Alt+E` torna ai pannelli e `Ctrl+R`
nel pannello controlla di nuovo. Se l'installazione non riesce, il terminale dice dov'è la
pagina di download del prodotto; dove nessun gestore di pacchetti di questa macchina risulta
avere il programma, `F4` propone di aprire quella pagina. Un nome di pacchetto si usa solo dove
è sicuro: uno sbagliato è peggio di nessuno.

Perché ognuno è un programma a parte e non è dentro fcmd:

| Programma | Serve per | Perché non è integrato |
| --- | --- | --- |
| `7z` | gli archivi che fcmd non legge da sé: rar, cab, iso, lha, arj, msi, wim, dmg… | il codice che spacchetta RAR ha una licenza sua, non aperta, e gli altri sono decine di formati che un programma legge già bene. Lo guida una tabella (`[[archiver]]`, *Formati tuoi*) e gli si chiede soltanto di elencare ed estrarre |
| `git` | il pannello Git (`Ctrl+Alt+G`) e il ramo nel prompt | cosa contiene un repository è quello che dice `git`; una seconda implementazione dentro fcmd potrebbe non essere d'accordo con la prima |
| `javap`, `ilspycmd`, `python3`, `wasm2wat`, `objdump`, `retdec-decompiler` | `Shift+F6` nel visualizzatore, tramite le regole `[[decompile]]` | ognuno è il lavoro di uno specialista per un tipo di codice; fcmd disassembla da sé il codice macchina e lascia a loro la decompilazione |
| `ffmpeg` | i fotogrammi di un video nel visualizzatore e nella vista rapida (`F3`, `Ctrl+Q`), e la sua riproduzione senza audio | i codec video sono coperti da brevetti, le librerie che li decodificano sono LGPL o GPL e non c'è ancora un decoder pronto in Rust puro; senza, un video si apre come i suoi byte e `F11` ne legge comunque il contenitore |
| `xdg-open` / `open` | aprire un link nel browser | quale browser è una scelta del desktop, non di fcmd |
| `journalctl` / `log` / `wevtutil` | *Comandi → Log di sistema* | ogni sistema ha un solo strumento che conosce il suo log |
| `systemctl` / `launchctl` | il pannello dei servizi | solo il gestore dei servizi può avviare e fermare un servizio |
| `plocate` o `locate` / `mdfind` / `es` | l'indice che `Alt+F7` può interrogare invece di scorrere il disco (*Trovare le cose*) | l'elenco dei file di un disco è il sistema a tenerlo aggiornato; un secondo elenco nostro sarebbe un secondo servizio da far girare. Mai necessario: senza, la ricerca scorre il disco |

Un programma conta come installato quando è nel `PATH`, con la stessa regola della shell, e da
nessun'altra parte. L'installatore di 7-Zip per Windows, per esempio, mette `7z.exe` in
*Programmi* senza aggiungerlo al `PATH`: il pannello allora lo dà per mancante, e la soluzione
è aggiungere quella cartella al `PATH`. *Programmi esterni* nel dialogo `F9` di `Alt+F1`
(`drives.panels.tools`) toglie la riga; `Ctrl+7` lì nasconde tutte le righe dei pannelli.

# Immagini a pixel veri

`viewer.pixels` dice come il visualizzatore e la vista rapida disegnano un'immagine o il
fotogramma di un video. È una riga di *Opzioni → Impostazioni del visualizzatore* (*Immagini*)
e una riga di *Tutte le impostazioni*.

| Valore | Cosa viene disegnato |
| --- | --- |
| `auto` | il predefinito: pixel veri in un terminale che si sa che li disegna, mezzi blocchi in tutti gli altri |
| `kitty` | il protocollo grafico di kitty, qualunque cosa il terminale dica di essere |
| `iterm` | le immagini in linea di iTerm2 |
| `sixel` | sixel |
| `off` | mezzi blocchi, sempre |

`auto` si basa sull'ambiente che il terminale ha dato al programma — mai su una domanda fatta
al terminale, che potrebbe restare senza risposta e lasciare fcmd ad aspettare, o ricevere la
risposta tardi, in mezzo a quello che stai scrivendo:

| Terminale | Riconosciuto da | Disegnato con |
| --- | --- | --- |
| kitty | `KITTY_WINDOW_ID`, oppure `TERM=xterm-kitty` | kitty |
| Ghostty | `TERM=xterm-ghostty`, oppure `TERM_PROGRAM=ghostty` | kitty |
| iTerm2 | `TERM_PROGRAM=iTerm.app` | iterm |
| WezTerm | `TERM_PROGRAM=WezTerm` | iterm |
| foot, mlterm, contour | `TERM` | sixel, quando la dimensione di una cella è nota |
| dentro tmux, screen, zellij | `TMUX`, `STY`, `ZELLIJ`, `TERM=screen…` | mezzi blocchi |
| qualunque altro | — | mezzi blocchi |

Quattro terminali disegnano i pixel e da qui non si può garantire per loro, quindi sta a te
nominarli: **Windows Terminal** dalla 1.22 disegna `sixel` e non dice che versione è; **xterm**
disegna `sixel` solo se avviato come VT340 (`xterm -ti 340`); il terminale di **VS Code**
disegna `iterm` solo con la sua impostazione *Enable Images* accesa; **Konsole** disegna
`kitty`, ma quanto di quel protocollo dipende da più cose che dalla sua versione. Un nome che si rivela sbagliato si
vede come un vuoto dove dovrebbe esserci l'immagine, o come una riga di lettere: rimettilo a
`off`.

A kitty e a iTerm2 viene detto quante celle copre un'immagine, e non serve altro. Sixel è
disegnato pixel per pixel, quindi fcmd deve sapere **quanto è grande una cella**. Lo chiede al
sistema — la dimensione della finestra del terminale, dal kernel, non dal terminale — e dove
quello non dice niente prende otto per sedici, oppure il dieci per venti che Windows Terminal
usa sempre per sixel. `viewer.cell_width` e `viewer.cell_height` lo dichiarano al suo posto,
per un terminale in cui un'immagine sixel esce di una dimensione diversa dalla cornice che le
è stata lasciata; vanno impostati tutti e due, e `0` vuol dire "chiedi".

```toml
[viewer]
pixels = "sixel"     # auto | kitty | iterm | sixel | off
cell_width = 9       # 0 = chiedi al sistema
cell_height = 18
```

# Cosa conserva una copia

```toml
[operations]
preserve = ["mtime", "mode", "xattrs"]
```

`operations.preserve` elenca cosa una copia conserva di un file oltre ai suoi byte: `mtime` è
la data dell'ultima scrittura, `mode` i permessi, `xattrs` gli attributi estesi — le etichette
che scrive un desktop (il capitolo sui file dice quali, sistema per sistema). Si conservano
tutti e tre a meno che l'elenco dica meno, e un elenco vuoto non conserva niente. Ognuno è una
casella del dialogo di `F5`, che si apre come dice questo elenco: una singola copia può sempre
fare diversamente. Dalla riga di comando si scrive con le virgole:
`--set operations.preserve=mtime,mode`.

# Una copia che non ricorda niente

`fcmd --no-save` parte con le tue cronologie e i pannelli come al solito ma non scrive niente
quando esce — per una seconda copia aperta per un lavoro veloce, che altrimenti, chiudendosi per
ultima, sovrascriverebbe le cronologie della copia principale con le sue.

# Cosa viene ricordato

Fra una sessione e l'altra il programma tiene, nel suo file di stato accanto alle impostazioni:
le righe di comando eseguite (`Alt+F8`), dov'era stato ciascun pannello (`Alt+F12`), **i file
che hai letto o modificato** (`Alt+F11`), le cartelle che tieni a portata di mano, le linguette
di entrambi i pannelli e dove erano. `ui.command_history` e `ui.file_history` dicono quanti
tenere del primo e del terzo elenco, e `ui.history_lifetime` quanti giorni si tiene una riga
che nessuno ha fissato dopo l'ultima volta che è stata usata — novanta di default, `0` per
tenerle per sempre. `Ins` in uno qualunque dei tre elenchi fissa la riga sotto il cursore, e
niente spinge fuori una riga fissata.

Un file che hai già letto si riapre dove l'avevi lasciato — la posizione, la code page, il
modo, l'andata a capo e i dieci segnalibri — a meno che non togli *Ricorda posizione e
codifica di ogni file* in *Opzioni → Impostazioni del visualizzatore*. Si tengono gli ultimi
cinquecentododici file.

L'elenco dei file viene scritto ogni volta che un visualizzatore o un editor si apre, quindi è
lo stesso elenco qualunque tasto abbia aperto il file. Un file su un server è ricordato per il
suo percorso remoto, non per la copia che è stata scaricata per leggerlo; un file riaperto
sale in cima invece di comparire due volte. `Enter` su una riga lo legge, `F4` lo modifica, e
una riga il cui file non c'è più lo dice.

# I tasti

La mappatura dei tasti è un profilo più le tue modifiche. Ne arrivano due: `far`, che è quello
per default, e `mc`. Le tue modifiche sono una mappatura tua dentro `fcmd.toml`, costruita su
una delle due e nominata in `[ui]`:

    [ui]
    keymap = "mia"

    [[keymap]]
    name = "mia"
    inherit = "far"

    [[keymap.bind]]
    keys = ["Ctrl+D"]
    scope = "panel"
    cmd = "file.delete"

    [[keymap.unbind]]
    keys = ["Ctrl+Q"]
    scope = "global"

Ogni `[[keymap.bind]]` aggiunge o sostituisce un'assegnazione, e `[[keymap.unbind]]` ne toglie
una del profilo. `scope` è dove vale — `global`, `panel`, `viewer`, `editor` o `dialog` — e `cmd` è uno dei nomi
nella pagina "Tutti i comandi" di questo aiuto. `fcmd --check-keymap` dice se una mappatura sta
in piedi e nomina tutto quello che è assegnato due volte.

Prima di riassegnare qualcosa vale la pena sapere cosa manda davvero il tuo terminale:
*Opzioni → Impara i tasti* disegna una casella per ogni tasto che vale la pena provare — i
tasti funzione con ogni modificatore, le frecce con `Ctrl` e `Shift`, i tasti di modifica — e
accende la casella quando quel tasto arriva. Premili e guarda. Una casella che non si accende
mai è un tasto che il tuo terminale non sta mandando affatto, e nessuna assegnazione lo farà
comparire; la riga sotto dice a cosa è legato l'ultimo tasto che invece è arrivato, che è come
si distingue "il terminale l'ha mangiato" da "è arrivato e non fa niente". Due `Esc` escono,
perché un `Esc` è un tasto sotto prova come tutti gli altri.

Midnight Commander, da cui questa cosa viene, ti lascia anche scrivere a mano la sequenza di
escape che un tasto avrebbe dovuto mandare. Qui no: le sequenze le legge una libreria che ha
una tabella per i terminali in uso, e lo fa meglio di qualunque cosa scritta a mano — quindi
quello che qui manca manca al terminale, e la risposta è un tasto diverso, non una tabella
diversa.

# I colori

Nove temi arrivano col programma:

- `commander` — il blu classico dei commander ortodossi, quello di partenza.
- `far` — la tavolozza di FAR Manager, presa dal suo codice.
- `black` — nero dietro a tutto, testi chiari, il cursore in grassetto e sottolineato.
- `white` — una pagina chiara per una stanza luminosa.
- `matrix` — verde su nero.
- `tron` — linee ciano su una notte blu scuro.
- `blade` — ambra e verde acqua sotto la pioggia.
- `github-dark` — i colori del terminale scuro di GitHub.
- `delphi-dark-2027` — l'editor scuro dell'IDE di Delphi, per tutto il programma, linee dei blocchi comprese.
- `delphi-inspiration` — i pannelli, i menu e i dialoghi blu classici di FAR, con l'editor scuro
  dell'IDE di Delphi nel visualizzatore (`F3`) e nell'editor (`F4`).

`Ctrl+Shift+T` passa dall'uno all'altro mentre il programma gira; `F9` → Opzioni → Temi li
offre come elenco, e `Shift+F9` tiene la scelta. In `fcmd.toml` è `theme = "far"` sotto `[ui]`,
e `fcmd --theme far` ne sceglie uno per un'esecuzione sola. `fcmd --list-themes` dice quali temi
ci sono e da dove arriva ciascuno. Il README del progetto ha un'immagine di ognuno, tutti sulla
stessa scena.

Uno tuo è un blocco `[[theme]]` di nomi di colore in `fcmd.toml`, oppure arriva con un plugin, e
viene trovato all'avvio — non c'è niente da registrare e niente da ricompilare. Il modo più
rapido per cominciare è scrivere fuori uno di quelli già fatti: `fcmd --export-theme tron >>
~/.config/fcmd.toml`, poi cambiare `id`, `name` e i colori che vuoi diversi. Un blocco può fare
`inherit` di un tema incluso e nominare solo quello che cambia, e uno con l'id di un tema incluso
lo sostituisce.

Il tema per default è la tavolozza di FAR, presa dal suo codice e non da uno screenshot.

Oltre ai colori, un tema può rendere il testo di una chiave grassetto, corsivo o sottolineato,
in una tabella `[styles]` sua:

```toml
[theme.styles]
"syntax.keyword" = "bold"
"syntax.comment" = "italic"
"cursor.fg" = "bold underline"
```

# Colorare i file per quello che sono

Un tema dice che aspetto ha un archivio o un file nascosto. Un gruppo `[[highlight]]` in
`fcmd.toml` lo dice per quello che vuoi tu: una maschera, una dimensione, un'età, gli
attributi, file o cartelle — scritti come li scrive il dialogo dei filtri.

```toml
[[highlight]]
name = "Rust"
mask = "*.rs"
fg = "#d7875f"

[[highlight]]
name = "Scritti oggi"
age = "<=1"
fg = "brightgreen"

[[highlight]]
name = "Grandi e nascosti"
size = "1G-"
attrs = "h-s"
fg = "red"
bg = "idx:52"
```

Decide il primo gruppo che corrisponde a una riga, quindi l'ordine nel file è l'ordine che
conta, e i gruppi predefiniti vengono dopo i tuoi. L'inchiostro è un colore — `#d7875f`, `red`,
`idx:208` — oppure il nome di un colore del tema come `file.archive`, che poi segue il tema
quando lo cambi. Un gruppo che non si legge viene nominato nella schermata di avvio e saltato;
il programma parte in ogni caso.

Non sei obbligato a scrivere il file. `F9 → Opzioni → Evidenziazione dei file` elenca i gruppi
nell'ordine in cui vengono provati, ogni riga disegnata con i suoi colori: `Enter` o `F4` ne
modifica uno, `Ins` ne crea uno nuovo accanto, `Del` ne toglie uno, e `Ctrl+↑`/`Ctrl+↓` ne
spostano uno oltre il vicino quando due corrispondono allo stesso file. Ogni riga del dialogo
prende esattamente quello che prende il file, e una condizione che non si legge torna indietro
con il problema nominato invece di essere tenuta. Una modifica vale subito sui pannelli;
`Opzioni → Salva la configurazione` è quello che la scrive nel file.

# Colonne tue

Le dieci viste sono elenchi di colonne che ha deciso qualcun altro. Un blocco `[[column_set]]`
ne rende tua una: l'`id` è la cifra su cui sta la vista, quindi `id = 3` è quello che disegna
`Ctrl+3`, e una vista per cui nessuno ha scritto un set resta quella predefinita.

```toml
[[column_set]]
id = 3
columns = [
  { type = "name",  width = "*" },
  { type = "size",  width = 10, align = "right", fmt = "bytes" },
  { type = "mtime", width = 16, fmt = "%Y-%m-%d %H:%M" },
]
```

I tipi sono `name`, `ext`, `size`, `allocated`, `mtime`, `btime`, `atime`, `attrs`, `owner`,
`group`, `links`, `descr` e `git` — gli stessi campi che disegnano le viste predefinite, e `git`,
che dice a che punto è ogni riga nel suo repository git: `M` o ` M` modificato (in staging a
sinistra, non ancora a destra), `A` aggiunto, `??` nuovo, `!!` ignorato, e per una cartella `M`
quando qualcosa dentro è cambiato. `{ type = "git", width = 2 }` accanto al nome è la colonna
che File Ninja chiama stato git; *ordina per stato git* in `Ctrl+F12` mette prima i file
cambiati. Una larghezza
è un numero di colonne, una percentuale del pannello, `*` per prendere quello che resta, oppure
`0` per la larghezza naturale della colonna, che per un nome o una descrizione vuol dire
"quello che resta" perché nessuno dei due ha una larghezza propria. `align` è `left` o `right`
dove l'abitudine della colonna non è quella che vuoi; `fmt` è `human` o `bytes` su una
dimensione — `1.18 M` contro `1 234 567` — e una stringa `strftime` su una data.

Una colonna per cui il pannello non ha spazio viene lasciata cadere da destra, e un set che
lascerebbe il nome illeggibile ne lascia cadere un'altra, che è la regola che la vista
dettagliata segue già con le sue date in più. Una colonna che costa una interrogazione per file
— il proprietario, il gruppo, il numero di nomi, la descrizione — viene recuperata dallo stesso
worker che usa la vista che la mostra, quindi si riempie un momento dopo l'elenco; una vista il
cui set non ne chiede nessuna non paga niente.

Un plugin può aggiungere colonne sue — il numero di righe, che tipo di file è davvero, la
data di una foto — e un set ne nomina una con il plugin e la colonna insieme:
`{ type = "plugin:line-count.lines" }`. *Opzioni → Configurazione dei plugin*, `F3` su un
plugin, elenca le colonne che ha, scritte così. Le sue celle mostrano `…` finché il programma
del plugin non ha risposto e poi vengono ricordate, così di un file si chiede una volta sola e
di nuovo solo quando cambia. Il capitolo *Plugin* più sotto dice come se ne scrive una.

Venendo da FAR puoi scrivere invece la coppia di stringhe che scrive il suo dialogo delle
modalità del pannello:

```toml
[[column_set]]
id = 4
far_types  = "N,SC,DM"
far_widths = "0,9,16"
```

Quello che FAR ha e questo programma no — la dimensione dello stream, l'ora di modifica dei
metadati, il numero di stream — viene nominato nella schermata di avvio invece di essere
lasciato fuori in silenzio, e un set in cui non c'è niente di leggibile lascia in pace la vista
predefinita di quel tasto.

Accanto alle colonne ci sono gli interruttori dello stesso dialogo di FAR, che appartengono
alla modalità e non al programma:

```toml
[[column_set]]
id = 4
columns = [{ type = "name", width = "*" }, { type = "size", width = 10 }]
fullscreen              = true   # questa vista dà al pannello tutto lo schermo
align_extensions        = true   # l'estensione di un file a destra del campo del nome
align_folder_extensions = false  # e quella di una cartella: spento, il punto fa parte del nome
folders_upper           = false  # nomi delle cartelle in maiuscolo
files_lower             = false  # nomi dei file in minuscolo
upper_to_lower          = false  # un nome *tutto* maiuscolo disegnato in minuscolo
```

Ognuno lasciato fuori è quello che il pannello disegnava prima che esistessero: le estensioni
dei file allineate, e nient'altro. `fullscreen` è il `Ctrl+F1` fatto dalla modalità — scegliere
la vista nasconde l'altro pannello e uscirne restituisce lo schermo — ma un pannello che hai
nascosto tu con il tasto resta com'è. `upper_to_lower` è per i nomi 8.3 che restituisce un
volume FAT, tutti maiuscoli e mai pensati per essere letti così; un nome senza lettere non è
tutto maiuscolo, quindi `README` cambia e `2026` no.

# Cosa è un file

`panel.descriptions` decide se il `descript.ion` della cartella venga letto o no. Acceso per
default: `Ctrl+Z` scrive una riga su un file, `Ctrl+6` e `Ctrl+7` le mostrano, e il pannello
informativo ne riporta una per esteso. Spento, il file non viene mai aperto — che è quello che
vuole chi non tiene descrizioni su una condivisione di rete, dove un file di servizio per
cartella è un giro di rete per cartella.

Due impostazioni di FAR dicono dove sta quel file e come viene scritto:

```toml
[panel]
description_names = ["files.bbs", "descript.ion"]   # quali nomi, e in che ordine
description_utf8  = false                           # tieni la code page in cui era
```

I nomi vengono cercati in ordine e il primo è quello sotto cui viene scritto un file nuovo;
una lista vuota sono i due qui sopra. Con `description_utf8` spento il file torna **nella
code page in cui è stato letto** — non c'è altro su cui basarsi, e indovinare quella della
macchina trasformerebbe un `descript.ion` russo in uno latino. Un file che ancora non c'è,
uno che era già UTF-8 e uno la cui pagina non può contenere quello che hai scritto vengono
scritti in UTF-8 lo stesso: un file che un programma DOS non legge è meglio di una riga che
ha perso le parole.

# Rompere la parità

`panel.sort_chain` dice cosa risolve due righe che la chiave di ordinamento lascia pari:

```toml
[panel]
sort = "size"
sort_chain = ["ext", "name"]
```

Fino a tre chiavi — `name`, `ext`, `size`, `mtime`, `btime`, `atime` — interrogate nell'ordine
in cui sono scritte. Il nome ha comunque l'ultima parola dopo di esse, così l'ordine non cambia
fra due letture della stessa cartella, e invertire l'ordinamento inverte tutto l'insieme invece
del solo primo passo.

# L'ordine dei nomi

```toml
[panel]
natural_sort = true     # img2 prima di img10
sort_case = "auto"      # auto | yes | no
```

`sort_case` dice se l'ordinamento distingue le maiuscole dalle minuscole. `auto` fa quello che
fa il gestore di file del sistema: su Linux contano, perché `Makefile` e `makefile` sono due
file, su Windows e macOS no, perché sono uno solo. `yes` e `no` lo dicono senza mezzi termini,
per una configurazione che deve ordinare allo stesso modo su ogni macchina. Quando contano, le
maiuscole vengono prima, come in `ls`.

Gli accenti latini sono tolti in ogni caso, e non c'è un'impostazione per questo: `é` si
ordina come `e`, `ß` come `ss`, `æ` come `ae`, `ø` come `o`, `ł` come `l` — ogni lettera di
Latin-1 e di Latin Extended-A, e anche un accento scritto come carattere a sé dopo la sua
lettera, che è come li conserva macOS. Due nomi che si leggono uguali sono decisi da come sono
scritti, prima quello senza accenti. È una tabella di lettere e non una lingua: non si
consulta l'alfabeto di nessun paese, e niente di ciò che sta fuori dal latino cambia posto.
Tutte e due le impostazioni sono righe di `F9 → Opzioni → Impostazioni di sistema`, e
di *Tutti i parametri*.

# Mettere davanti certi tipi di file

Un blocco `[[sort_group]]` in `fcmd.toml` è una fascia dell'elenco: una maschera, e da quale
parte dei file normali sta. Le fasce tirano certi tipi di file davanti all'elenco e ne spingono
altri dietro, qualunque sia l'ordinamento del pannello.

```toml
[[sort_group]]
name = "sorgenti"
mask = "*.rs,*.toml"

[[sort_group]]
name = "prodotti della compilazione"
mask = "*.o,*.rlib,*.d"
place = "back"
```

Le fasce davanti tengono l'ordine in cui sono scritte, così quelle dietro, e i file che non
corrispondono a nessuna fascia stanno in mezzo. Dentro una fascia vale ancora l'ordinamento
normale, le cartelle tengono il loro posto in cima, e `reverse` inverte ogni fascia invece di
scambiare le fasce fra loro. Scrivere i blocchi le accende; `Shift+F11` — `Gruppi` sulla barra
dei tasti — le spegne e le riaccende, un pannello per volta. Un blocco che non si legge viene
nominato nella schermata di avvio e saltato, come un gruppo di evidenziazione.

# Sincronizzazioni conservate per nome

Un blocco `[[sync]]` in `fcmd.toml` è una coppia di cartelle e il modo di sincronizzarle, sotto
un nome. Il dialogo della sincronizzazione ne scrive uno quando dai un nome alla coppia, e
`Ctrl+Alt+Y` li elenca per rieseguirli; si possono anche scrivere a mano.

```toml
[[sync]]
name = "Backup della sera"
left = "~/lavoro"
right = "/media/chiavetta/lavoro"
recursive = true
both_ways = false
delete_extra = true
by_content = false
filter = "*.rs,*.md"
report = "~/resoconto-backup.txt"
```

`left` è l'origine quando si va in un verso solo. `recursive` scende nelle sottocartelle ed è
attivo se manca; `both_ways`, `delete_extra` (offri di togliere quello che l'altro lato non ha)
e `by_content` (confronta i byte invece di dimensione e data) sono spenti. `filter` è una
maschera e `report` il file in cui un'esecuzione scrive quello che ha fatto; entrambi si possono
omettere. Un blocco è una domanda, non un ordine: eseguirlo mostra la tabella, ed è `F2` lì a
far partire qualcosa.

# L'editor della tavolozza

`F9 → Opzioni → Colori…` elenca ogni colore che l'interfaccia usa, con il valore come lo
scrive un file di tema e due suoi campioni: il colore come inchiostro e il colore come sfondo.
`Enter` chiede quello nuovo — un nome (`red`, `brightcyan`, `default`), uno dei 256
(`idx:208`), oppure `#ff8000` — e lo schermo cambia mentre rispondi, che è l'unico modo di
giudicare un colore.

`F2` scrive il risultato in `fcmd.toml` come blocco `[[theme]]` sotto l'id del tema, così
l'avvio successivo parte con quello e `Shift+F12` continua a girare fra i temi. Uscire senza
`F2` tiene i colori per questa sessione e lo dice.

# Il tuo menu

`F2` apre un menu che scrivi tu. Sta in `fcmd-menu.toml` accanto alle impostazioni —
`fcmd --paths` dice esattamente dove — e una riga è un'etichetta, un tasto con cui prenderla, e
le righe di comando da eseguire:

    [[menu]]
    key = "c"
    label = "Compila (cargo)"
    when = "panel.has('Cargo.toml')"
    run = ["cargo build --release"]

    [[menu]]
    key = "g"
    label = "Git"
    submenu = [
      { key = "s", label = "stato", run = ["git status -sb"] },
      { key = "l", label = "storia", run = ["git log --oneline -40"] },
    ]

Una riga è la stessa riga di comando che prende la riga di comando stessa, quindi le
sostituzioni funzionano: `{file}` è il file sotto il cursore fra apici, `{selection}` i nomi che
hai scelto, `{list}` un file che li contiene, `{ask:title=Ramo}` ti chiede qualcosa prima che
parta tutto. Più righe vengono eseguite una dopo l'altra e si fermano al primo errore.

`when` decide se una riga venga mostrata. Quattro fatti — `panel.has('Cargo.toml')`,
`file.is('*.rs')`, `panel.selected`, `panel.local` — con `!`, `&&` e `||` fra di essi. Una
condizione che non riesce a leggere lascia la riga sullo schermo e dice perché, invece di
nasconderla in silenzio.

Una riga con `separator = true` è invece una linea fra due gruppi, e un `label` su di essa
viene scritto nella linea come titolo del gruppo. Un separatore il cui gruppo è stato svuotato
dalle condizioni se ne va con esso, così il menu non mostra mai due linee di fila.

    [[menu]]
    separator = true
    label = "Controllo versione"

`Shift+F10` riapre il menu sulla voce che hai scelto l'ultima volta — dentro il suo sottomenu,
se era in uno — così eseguirla di nuovo è `Shift+F10` e `Invio`.

Una cartella può portarsi il suo `.fcmd-menu.toml`, e le sue righe stanno sopra quelle che
valgono sempre: il menu del progetto in cui sei è quello che vuoi mentre ci sei dentro.

**Venendo da FAR** non devi scrivere prima niente di tutto questo: un `FarMenu.ini` viene
letto così com'è — metti quello del tuo profilo di FAR accanto a `fcmd.toml`, e quelli nelle
cartelle dei tuoi progetti vengono trovati dove già sono. I tasti rapidi, le etichette, i
sottomenu tra `{` e `}` e i separatori `--` diventano le righe qui sopra, e i simboli di FAR
nei comandi diventano i nostri: `!.!` il nome, `!` il nome senza estensione, `!\` la
cartella, `!@!` e `!&` la selezione come file e come nomi, `!#` l'altro pannello,
`!?titolo?iniziale!` una domanda. Le forme con i nomi corti (`!~`, `!-!`) vengono lette come
quelle lunghe. Una riga che usa qualcosa che qui non ha un corrispondente — la lettera del
disco `!:`, la selezione dell'altro pannello — resta nel menu e non esegue niente: `Invio`
su di essa dice perché. Quando qualcosa è stato letto diversamente da FAR la riga di stato lo
dice all'apertura del menu, e l'ultima riga del menu elenca ogni osservazione.

Dove ci sono tutti e due i file nello stesso posto, il menu è `fcmd-menu.toml` e
`FarMenu.ini` non viene letto: non vengono mai mescolati, così convertire un menu a mano non
mostra ogni riga due volte.

# Plugin

Un plugin è una cartella dentro `fcmd-plugins/`, accanto alle impostazioni (`fcmd --paths` dice
dove), con dentro un `plugin.toml`. Viene trovato all'avvio: niente da registrare, niente da
compilare. Può portare **dati** — temi, keymap, blocchi `[[assoc]]`, regole `[[decompile]]`,
liste `[[mask-group]]`, con le stesse parole di `fcmd.toml` — e **comandi**: un `[[command]]`
nomina un programma nella cartella del plugin, e `F11` elenca i comandi per lo schermo in cui
sei — nei pannelli anche `Ctrl+Shift+F11`, che la barra dei tasti mostra mentre li tieni premuti.

`F11` elenca tutti i comandi, prima quelli che possono girare dove sei. Gli altri stanno sotto,
in grigio, e ciascuno dice perché — un altro schermo, un plugin spento, o una condizione che il
suo autore ha scritto in `[command.when]`: una cartella su questa macchina (`local`), file su cui
lavorare (`files`), file che corrispondono a delle maschere (`masks`), un nome in questa cartella
(`here`) o in questa o in una sopra (`inside`, `[".git"]` per tutto quello che riguarda un
repository). `Invio` su una riga grigia ripete il motivo e non esegue niente. Senza plugin
installati, `F11` propone di aprire la cartella dei plugin e questa pagina.

Un comando gira come programma a sé, mai dentro fcmd, così un plugin che va in crash ci va da
solo. Riceve dove sono i pannelli e cosa è selezionato, nell'ambiente e come JSON sul suo
input, e risponde una riga alla volta: un messaggio, una cartella in cui andare, file da
selezionare, o un comando di fcmd da eseguire — niente che un tasto non possa fare. Un comando
che rinomina o sovrascrive file lo dichiara nel manifest, e fcmd chiede prima di eseguirlo. In
`examples/plugins/` ce ne sono ventuno di esempio, dai file duplicati alla conversione dei
fine riga.

Un plugin può anche calcolare una **colonna** per i pannelli. Un `[[column]]` nel suo
`plugin.toml` nomina un programma:

```toml
[[column]]
id = "lines"
title = "Lines"
width = 7
align = "right"
run = "count.sh"
masks = "*.rs,*.toml"
timeout = 10
```

e `{ type = "plugin:<plugin>.lines" }` in un `[[column_set]]` (vedi *Colonne tue*) la
disegna. Il programma viene eseguito una volta per file, con il percorso come primo
argomento, e la prima riga che stampa è la cella; niente in uscita, o uno stato di uscita di
errore, è una cella vuota. `masks` dice di quali file gli si chiede (tutti se manca; le
cartelle solo con `folders = true`), e `batch = true` lo esegue invece una volta per cartella
— i percorsi sul suo input, uno per riga, e `percorso<TAB>valore` per ciascuno in uscita — per
un programma lento a partire. Niente di questo ferma il pannello: l'elenco viene disegnato
subito, con `…` nelle celle di cui si sta ancora chiedendo, e si riempie man mano che
arrivano le risposte. Una risposta viene conservata per percorso, data e dimensione per il
resto della sessione. Un'esecuzione che dura più di `timeout` secondi (dieci se manca) viene
fermata; dopo tre di queste in una cartella la colonna lì rinuncia e la riga di stato lo
dice. Si chiede solo per le cartelle sui dischi di questa macchina — dentro un archivio o su
un server la colonna resta vuota — e il pannello non si può ordinare per una colonna così.
Le colonne di un plugin compaiono, o spariscono, all'avvio successivo a quando lo si attiva o
disattiva.

*Opzioni → Configurazione dei plugin* elenca tutti i plugin trovati: `✓` per uno attivo, `·`
per uno disattivato e `!` per uno che non si è caricato. `Invio` attiva o disattiva quello
sotto il cursore e lo scrive subito (`[plugins] disabled` in `fcmd.toml`); i suoi comandi
diventano grigi in `F11`, o tornano normali, immediatamente, mentre il resto di quello che porta — temi, tasti,
associazioni, decompilatori, gruppi di maschere — si legge all'avvio e cambia al prossimo. `F3`
è la sua scheda: chi l'ha scritto, dove sta, cosa porta, i suoi comandi, le sue colonne e quello che è stato
detto mentre si caricava — per uno che non si è caricato, il perché. Le sue impostazioni sono
file suoi: `F3` dice la cartella, e il `plugin.toml` del plugin è quello da modificare. `fcmd
--list-plugins` stampa cosa è stato trovato, e cosa non si è caricato, senza aprire l'interfaccia.

# Formati tuoi

Un blocco `[[archiver]]` in `fcmd.toml` insegna ai pannelli un formato che nulla di ciò che è
incluso sa leggere: nomina un programma della macchina, la riga di comando che elenca un
archivio e quella che ne estrae una voce, e come leggere quello che l'elenco stampa. La lingua è
quella di MultiArc di FAR, quindi una tabella copiata da un `custom.ini` tiene i nomi delle sue
chiavi.

    [[archiver]]
    name = "unace"
    extensions = ["ace"]
    List = "v -y %A"
    Extract = "x -y %A @%L"

    [archiver.listing]
    start = "^Date    Time"
    end = "^listed:"
    rows = ['^(?<size>\d+)\s+(?<name>\S.*)$']

`%A` è l'archivio e `%L` un file con i nomi da estrarre. Una riga di comando non viene mai data
a una shell: è divisa sugli spazi una volta sola, e un nome con uno spazio resta un argomento
solo. Le righe dell'elenco sono espressioni con catture con nome, e `name` è quella che non può
mancare.

Si accetta solo la lettura — `List`, `Extract`, `ExtractWithoutPath`, `Test`. Un blocco con una
riga che cambierebbe un archivio (`Add`, `Delete`, `SFX`…) viene rifiutato per nome, e così un
blocco sbagliato in qualunque altro modo: ciascuno è un avviso all'avvio, con il nome del blocco
e il motivo, invece di un archivio rotto al primo `Enter`. I tuoi blocchi sono interrogati prima
di quello `7z` incluso, e un blocco chiamato `7z` lo sostituisce: è così che se ne cambiano le
opzioni. Un formato che questo programma legge da sé — zip, 7z, i tar, `ar`, i compressori
nudi — non viene mai passato a un blocco.

