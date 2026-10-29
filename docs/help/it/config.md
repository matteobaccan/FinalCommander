# Dove sta ogni cosa

Tutto è un file che puoi leggere, confrontare e copiare su un'altra macchina:

    config.toml     le impostazioni
    keymap.toml     le tue assegnazioni di tasti, sopra il profilo
    session.toml    dov'erano i pannelli
    themes/         gli schemi di colore
    sites.toml      le connessioni salvate

Stanno dove la piattaforma tiene queste cose — `~/.config/fcmd` su Linux, `Library/
Application Support` su macOS, `%APPDATA%` su Windows. `fcmd --dump-config` stampa le
impostazioni con cui il programma sta davvero girando, che è il modo più rapido di vedere come
si chiama una chiave.

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
per default, e `mc`. Una tua assegnazione è una riga in `keymap.toml`:

    [[bind]]
    keys = ["Ctrl+D"]
    scope = "panel"
    cmd = "file.delete"

`scope` è dove vale — `global`, `panel`, `viewer`, `editor` o `dialog` — e `cmd` è uno dei nomi
nella pagina "Tutti i comandi" di questo aiuto. `fcmd --check-keymap` dice se una mappatura sta
in piedi e nomina tutto quello che è assegnato due volte.

# I colori

`Ctrl+Shift+T` passa da un tema all'altro; il menu li offre come elenco. Un tema è un file TOML
di nomi di colore, e uno messo dentro `themes/` viene trovato all'avvio — non c'è niente da
registrare e niente da ricompilare.

Il tema per default è la tavolozza di FAR, presa dal suo codice e non da uno screenshot.

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
`group`, `links` e `descr` — gli stessi campi che disegnano le viste predefinite. Una larghezza
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

Una cartella può portarsi il suo `.fcmd-menu.toml`, e le sue righe stanno sopra quelle che
valgono sempre: il menu del progetto in cui sei è quello che vuoi mentre ci sei dentro.
