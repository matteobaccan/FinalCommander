# Copiare, spostare, creare, cancellare

`F5` copia, `F6` sposta o rinomina, `F7` crea una cartella, `F8` cancella. La destinazione di
una copia o di uno spostamento è l'altro pannello, e il dialogo lo dice prima che accada
qualcosa.

Ognuna di queste è un lavoro: gira in sottofondo con una barra di avanzamento, `Esc` la ferma, e
quello che ha già fatto resta fatto. Un conflitto — un file che è già là — si ferma e chiede,
con le risposte "tutti" e "nessuno" perché una copia lunga non sia mille domande.

Il dialogo di copia ha dentro più della destinazione, e tutto il resto è opzionale: scrivi un
percorso, premi `Enter`, e niente sotto la prima riga si applica.

* **Se è già là**: chiedi, sovrascrivi, sovrascrivi quando l'origine è più nuova, salta, dai
  alla copia un nome libero, oppure aggiungi alla fine.
* **Solo i file più nuovi di quelli che ci sono** è una casella accanto a quella scelta e non
  uno dei suoi valori, e le due cose non sono la stessa: la casella vale anche quando la scelta
  dice sovrascrivi, e anche dopo che hai risposto "sovrascrivi tutto" a una domanda.
* **Solo i nomi che corrispondono** prende una maschera, così viaggia solo quello che
  corrisponde. Vale per i file e mai per le cartelle, e una cartella viene creata solo quando
  qualcosa sotto di essa ha corrisposto — così `*.rs` tirato fuori da un albero di sorgenti ti
  dà le cartelle che hanno dentro del Rust e nessun'altra.
* **Copia quello a cui punta un link, non il link** è spento, e deliberatamente: una copia che
  trasformasse i link in copie dei loro obiettivi duplicherebbe un albero la prima volta che ci
  provi.
* **Conserva gli orari** e **Conserva i permessi** sono accesi.

Qualunque cosa rispondi è quello su cui si apre il dialogo successivo.

La riga della destinazione prende più di una destinazione, separate da `;` — metti fra apici un
percorso che ha un `;` nel proprio nome. Vengono eseguite una dopo l'altra e non insieme, e solo
la prima di uno *spostamento* è uno spostamento: dopo di essa l'origine non c'è più, quindi le
altre sono copie di quello che è arrivato per primo. Fermarne una ferma anche quelle dietro.

`Shift+F5` e `Shift+F6` copiano e spostano *dentro la stessa cartella*, che è come si duplica o
si rinomina un file senza toccare l'altro pannello. `Shift+F8` cancella senza la domanda, e lo
dice nel proprio titolo.

`Alt+F6` crea un link. `Ctrl+G` esegue un comando su tutto quello che è selezionato, con il
nome di ciascun file messo nella riga dove lo segni tu.

# Le somme di controllo

*Somme di controllo…* nel menu File scrive un elenco di impronte per quello che è selezionato:
SHA-256, SHA-1 o MD5. Un elenco per tutto il gruppo — chiamato come la cartella, o come
qualunque altra cosa scrivi — oppure uno accanto a ciascun file. Il formato è quello che
scrivono `sha256sum` e compagni, così l'elenco può essere verificato da chiunque tu lo mandi con
lo strumento che ha già, e quello che ti mandano può essere verificato qui.

*Verifica contro un elenco* lavora sull'elenco sotto il cursore. Quale hash l'ha prodotto viene
ricavato dalle impronte che contiene e non dal suo nome, quindi un file chiamato `SHA256SUMS`,
`sums.txt` o `release.md5` funzionano tutti. Quello che non corrisponde viene nominato nel
rapporto **e lasciato selezionato nel pannello**, perché la cosa che fai dopo uno scaricamento
sbagliato è riscaricarlo o cancellarlo, e entrambe lavorano sulla selezione.

`Esc` ferma una verifica lunga.

# Cancellare, e il cestino

`F8` consegna quello che hai scelto al cestino del sistema — lo stesso che mostra il file
manager del tuo desktop, così quello che entra si può riprendere da là. Una cartella entra
intera. La domanda che fa dice *cestino* e non *cancella*, così le due cose non si confondono
mai.

`Shift+Del` cancella e basta, oltre il cestino, che è quello che quel tasto significa in ogni
altro posto. *Usa il cestino di sistema* nelle impostazioni di sistema spegne il cestino, e
allora `F8` è la stessa cosa.

Solo questa macchina ha un cestino. Un file su un server, una voce in un archivio e un oggetto
in un bucket vengono cancellati e basta, e il dialogo lo dice prima che tu risponda. Se il
cestino rifiuta qualcosa che avrebbe dovuto prendere — su Linux, un file su un disco che non ha
una propria cartella di cestino — l'errore dice quale tasto lo cancella invece.

`Alt+Del` è il terzo: sovrascrive i byte prima di scollegare il file. Il suo dialogo è onesto su
quanto valga la cosa su un SSD, su un filesystem copy-on-write e con gli snapshot in giro.

# Tenere due cartelle uguali

`Ctrl+Shift+Y` sincronizza le cartelle dei due pannelli. Succede in tre passi, e quello di mezzo
è il punto:

1. Un dialogo chiede come decidere — in un verso (il pannello sinistro è l'origine) o nei due
   versi con il file più nuovo che vince, se scendere nelle sottocartelle, se offrire di
   togliere quello che l'altro lato non ha, una maschera, e se confrontare i byte invece della
   dimensione e della data.
2. Una **tabella** di ogni coppia, con una freccia nel mezzo che dice da che parte quel file sta
   per andare. `Spazio` passa fra le risposte che una riga permette e scende, `←` e `→` dicono
   da che parte, `=` la lascia stare. Le righe che non faranno niente sono in grigio; un
   contrasto — stessa data e dimensione diversa, o un file contro una cartella — è marcato e non
   fa niente fino a che non scegli.
3. `F2` esegue. **Niente** viene scritto prima di quel momento, quindi la tabella è anche la
   prova a vuoto.

Due secondi di differenza contano come stesso file, perché è quello che un giro su una chiavetta
o dentro uno zip fa a una data, e trattarlo come un cambiamento copierebbe tutto ogni volta. Se
ti serve cogliere un cambiamento che ha tenuto la stessa dimensione e la stessa data, spunta
*Confronta i byte* — legge i due lati.

Le copie sovrascrivono senza chiedere: la tabella era la domanda, e tu hai risposto riga per
riga. Una riga che porta via un file lo mette nel cestino, come fa `F8`.

Per ora le due cartelle devono essere entrambe su questa macchina.

# Trovare le cose

`Alt+F7` cerca i file: una maschera, una dimensione, il testo dentro di essi, e dove guardare. I
risultati sono un elenco che puoi visualizzare, modificare o su cui saltare — e `F5` trasforma
tutto il risultato in un pannello, così tutto quello che funziona su una cartella funziona su
una ricerca.

La riga della dimensione è quella che trova i file che vale la pena cancellare quando un disco
si riempie: `100M` significa "da cento megabyte in su", `1M-1G` un intervallo, `-4K` tutto
quello che è più piccolo di quattro kilobyte. Lasciala vuota per qualunque dimensione. `K`, `M`,
`G` e `T` sono potenze di 1024, come in ogni altro punto del programma.

Sostituire attraverso i file è lo stesso dialogo con una sostituzione, e conta i file e le
occorrenze prima di scrivere qualunque cosa. Il conteggio viene dallo stesso motore che poi fa
il lavoro, quindi i due non possono essere in disaccordo.

# Gli archivi

`Shift+F1` comprime quello che è selezionato — e può cancellare gli originali una volta che
l'archivio è davvero scritto, che è una casella nel dialogo e mai il comportamento per default.
`Shift+F2` estrae l'archivio sotto il cursore in una cartella sua. `Shift+F3` è il menu degli
archivi: verificarlo (ogni voce viene decompressa e controllata, niente viene scritto),
estrarlo qui, estrarlo da un'altra parte, cancellarlo, o aggiungere file a uno nuovo.

`Enter` su un archivio lo apre come se fosse una cartella — zip, 7z, tar, e tar con gzip,
bzip2, xz o zstd — e il percorso dice dove sei:

    /home/io/sorgenti.tar.gz!/src/main.rs

Tutto quello che legge un file funziona là dentro: il visualizzatore, l'editor esadecimale, la
ricerca.

Anche scrivere funziona, e funziona nell'unico modo che questi formati permettono. Nessuno di
essi si può modificare sul posto — un `.tar.gz` è un unico flusso compresso da un capo
all'altro, e un 7z di solito impacchetta più file in un blocco solo — quindi un cambiamento
vuol dire riscrivere l'archivio: il vecchio viene riversato in un file nuovo accanto a esso con
la modifica applicata, il nuovo viene riletto e controllato, e solo allora sostituisce
l'originale. Un cambiamento interrotto lascia quindi l'archivio esattamente come era e un file
`.part` da cancellare. Le voci che nessuno ha toccato vengono copiate così come sono, come byte
già compressi, quindi tirare fuori un file da uno zip da cento megabyte è una copia e non una
ricompressione. Il 7z è l'eccezione: i suoi file condividono un blocco compresso, quindi
cambiarne uno decodifica e reimpacchetta anche gli altri, e questo è il formato, non noi.

Un 7z che questo programma *scrive* mette ogni file in un blocco suo. È più grande di quello che
scrive 7-Zip per default, e qui è il compromesso giusto: in un file manager l'atto comune è
leggere un file fuori da un archivio, e un archivio solido fa sì che questo significhi
decodificare tutto quello che gli sta davanti. Un 7z cifrato — anche uno con la testata cifrata,
dove non si vedono nemmeno i nomi — dice che vuole una password, che non c'è ancora dove
scrivere.

Quello che questo ti dà, da dentro un archivio:

* `F4` apre una voce nell'editor. Salvare la rimette dentro.
* `F6` rinomina una voce, o la sposta in un'altra cartella dentro lo stesso archivio. Il dialogo
  offre il nome che ha, perché una destinazione nell'altro pannello vorrebbe dire tirarla fuori
  dall'archivio, e quello è `F5`.
* `F8` tira fuori delle voci, selezione compresa. Una cartella si porta via il suo albero.
* `F5` e `F6` *dentro* un archivio — con l'altro pannello che sta dentro uno — ci mettono dei
  file. Un file per volta, quanti sono quelli selezionati; una cartella intera è `Shift+F1`, che
  la percorre in un lavoro. `F6` cancella gli originali una volta che l'archivio è scritto, e
  non prima.

Quello che non c'è: i permessi e gli orari di una voce non si possono cambiare dal pannello, e
`F7` non crea una cartella dentro un archivio — nessuno di questi formati conserva una cartella
vuota che valga la pena di tenere.

# Cosa è un file

`Ctrl+A` apre il dialogo degli attributi: quello che il file *è*, invece di quello che contiene.

Su Linux e macOS sono i permessi, come dodici caselle — proprietario, gruppo e tutti, ognuno con
lettura, scrittura ed esecuzione, più setuid, setgid e sticky — e la riga sopra di esse dice cosa
è il file ora, sia come `rw-r--r--` sia come `0644`. Su Windows sono invece sola lettura,
nascosto, sistema e archivio. La sola lettura è su entrambi, perché su Unix sono i bit di
scrittura.

Sotto le caselle stanno i due orari, come `2026-10-24 17:42:05` in ora locale. Un orario lasciato
in bianco viene lasciato stare; un orario scritto viene impostato. Con una cartella fra quello
che è selezionato c'è una casella in più: *e tutto quello che sta dentro le cartelle*, che porta
la modifica giù per l'albero. Le cartelle stesse vengono cambiate dopo quello che contengono,
così togliere il bit di scrittura a una cartella non può chiudere il programma fuori dal proprio
contenuto a metà strada.

Riferisce come una copia — una finestrella di avanzamento, un conteggio, `Esc` — e un file che
rifiuta viene elencato con la ragione invece di essere scavalcato in silenzio. Una cosa viene
lasciata stare invece di essere riferita: gli orari *propri* di una cartella su Windows, che non
si possono impostare senza aprire la cartella come handle. I file che stanno dentro ottengono
gli orari che hai chiesto. Un file su un server ottiene i permessi e la data di modifica, che è
quello che porta SFTP. Una voce dentro un archivio ha i permessi dell'archivio e un oggetto in un
bucket ha la policy del suo bucket, quindi nessuno dei due ha niente da impostare, ed entrambi lo
dicono invece di fingere.

Quello che non c'è ancora: il proprietario e il gruppo, e i bit avanzati di Windows.

## Confrontare due cartelle

`F9 → Comandi` ne ha quattro, e rispondono a domande diverse.

**Confronta le cartelle** è quello di FAR, ed è istantaneo: accoppia i nomi, confronta la
dimensione e l'orario, e lascia selezionato in ciascun pannello quello che l'altro lato non ha o
ha in una copia più vecchia. Quello che resta selezionato è quello che una copia dovrebbe
portare.

**Confronta per contenuto** legge i file. Ogni coppia con lo stesso nome viene aperta sui due
lati e i byte confrontati — una coppia le cui dimensioni sono già diverse è risolta senza aprire
niente, e le altre sono lette fino al primo byte che differisce, che per due file che non sono
uguali di solito sta nel primo blocco. È la domanda da fare su due alberi costruiti dallo stesso
sorgente, o copiati da qualcosa che non ha conservato le date, dove la data non dimostra niente.

**Confronta con le opzioni** è lo stesso lavoro con la domanda scritta per esteso: un dialogo su
cosa conta come differenza, che è l'unico modo onesto di confrontare due cartelle, perché non ci
sono due persone che intendano la stessa cosa per *uguale*. Se i nomi distinguano maiuscole e
minuscole. Se conti la dimensione, se conti l'orario e con quale precisione — due secondi è
quello che tiene FAT, e un file copiato su una chiavetta e indietro è sfalsato di due secondi
senza colpa di nessuno. Se leggere i byte, e se leggerli solo quando l'orario differisce, che è
il modo rapido di controllare un backup. Quanto leggerne: un primo frammento di tanti kilobyte
risolve una coppia di file grandi in una frazione del tempo, e dice che è quello che ha fatto. Se
due file che differiscono solo per i fine riga contino come diversi — `CRLF` contro `LF` è una
differenza nei byte e di solito non è una differenza nel file. Se guardare solo quello che è
selezionato, se fermarsi alla prima differenza, e se marcare solo il lato più nuovo o entrambi.

**Nelle sottocartelle**, che lo stesso dialogo accende, confronta due alberi interi: ogni file è
accoppiato per il suo percorso sotto le due radici, fino alla profondità che permetti, e la
risposta torna come un pannello a parte con ogni riga marcata per il motivo per cui è là —
differisce, è solo a sinistra, è solo a destra, oppure è uguale a meno dei fine riga. Una
differenza trovata sei cartelle più in basso non può essere una selezione in un pannello che
mostra una cartella, ed è per questo che è un pannello e non una selezione.

**Trova i duplicati** fa l'altra domanda: non se due cartelle corrispondano, ma quali file dentro
un albero sono lo stesso file sotto nomi diversi. Confronta quello che contengono — una
dimensione e poi un hash — così due nomi che non hanno niente in comune finiscono comunque
insieme, e la risposta è un pannello dei gruppi che ha trovato.

Per tutto questo i due pannelli devono stare su un filesystem locale, e qualunque cosa non abbia
una controparte sull'altro lato è una differenza. Un confronto lungo dice quanto è arrivato nella
riga di stato, e `Esc` su una riga di comando vuota lo ferma.

# Cosa apre un file

`Enter` su un file che non è un archivio e non è un programma esegue quello che dice di eseguire
un'associazione. Un'associazione è un blocco `[[assoc]]` in `fcmd.toml`, e sono tre comandi e una
maschera:

    [[assoc]]
    match = "*.md"
    name  = "Markdown"
    open  = "glow -p {file}"
    view  = "bat --style=plain {file}"
    edit  = "hx {file}"

La maschera è o un nome intero (`Makefile`) o `*.` e un'estensione (`*.md`), confrontata senza
badare alle maiuscole; vince il primo blocco che corrisponde, quindi quelli specifici vanno
prima. `{file}` è il percorso **fra apici** e `{file.raw}` è quello nudo — le stesse sostituzioni
che prendono il menu dell'utente e `Ctrl+G`, quindi `{name}`, `{dir}`, `{other.dir}`, `{list}` e
`{ask:…}` funzionano anche qui.

Senza nessun blocco su cui regolarsi, `Enter` consegna il file al desktop — `xdg-open` su Linux,
`open` su macOS, `start` su Windows — che è quello che `general.desktop_open` spegne per una
macchina su cui questa sarebbe la cosa sbagliata. Con anche quello spento, dice che non c'è niente
con cui aprire il file invece di tirare a indovinare.

`F3` e `F4` sono sempre *nostri*: il visualizzatore e l'editor di questo programma, qualunque cosa
dica un blocco. Un tasto che a volte aprisse qualcos'altro sarebbe un tasto di cui nessuno
potrebbe fidarsi. Quello che un blocco nomina per leggere e per scrivere sta su `Alt+F3` e
`Alt+F4` — i tasti di FAR per l'altro visualizzatore e l'altro editor — e se non c'è nominato
niente nemmeno là, quei due ricadono su `$PAGER` e su `$VISUAL` o `$EDITOR`, che su una macchina
che ha un'opinione sono già impostati. (Su alcuni desktop `Alt+F4` chiude la finestra prima che il
programma lo veda; quello è il tasto del desktop, e il menu Comandi ha la stessa voce.)

`Ctrl+PgGiù` resta il modo di entrare *dentro* un archivio e `Enter` su uno continua ad aprirlo
come cartella: un'associazione per `*.zip` fermerebbe tutto questo, quindi gli archivi vengono
controllati per primi.

**Apri con** — nel menu Comandi — elenca ogni blocco che corrisponde al file sotto il cursore,
più il desktop, ed esegue quello scelto. È il modo di usare il comando `view` di un blocco senza
riassegnare niente.
