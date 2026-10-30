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

`Tab` sulla riga della destinazione completa il percorso, come fa una shell: una sola
corrispondenza viene scritta per intero — con il separatore quando è una cartella, così il
`Tab` successivo ci entra — e più corrispondenze vengono prese fin dove sono d'accordo, con il
conto nella riga di stato. Quando non c'è niente da aggiungere, `Tab` va ai pulsanti come ha
sempre fatto. Lo stesso vale per la cartella di un'estrazione, il nome di un archivio e un file
di chiave.

La riga della destinazione prende più di una destinazione, separate da `;` — metti fra apici un
percorso che ha un `;` nel proprio nome. Vengono eseguite una dopo l'altra e non insieme, e solo
la prima di uno *spostamento* è uno spostamento: dopo di essa l'origine non c'è più, quindi le
altre sono copie di quello che è arrivato per primo. Fermarne una ferma anche quelle dietro.

`Shift+F5` e `Shift+F6` copiano e spostano *dentro la stessa cartella*, che è come si duplica o
si rinomina un file senza toccare l'altro pannello. `Shift+F8` cancella senza la domanda, e lo
dice nel proprio titolo.

`Alt+F6` crea un link. `Ctrl+G` esegue un comando su tutto quello che è selezionato, con il
nome di ciascun file messo nella riga dove lo segni tu.

*Usa la copia di sistema* nelle impostazioni di sistema è spenta per default, e vale la pena
saperlo se copi file grossi. Accesa, la copia la fa il kernel invece di questo programma che
legge i byte e li riscrive: su btrfs, XFS e APFS una copia dentro lo stesso filesystem diventa
un reflink — un secondo nome per gli stessi dati, scritto in un istante per quanto grande sia
il file — e su una condivisione di rete è il server a copiare il file, senza che i byte
attraversino la rete due volte. Il motivo per cui è spenta è l'altra metà della verità: su
macOS e Windows il sistema copia un file intero in una sola chiamata, quindi `Esc` riceve
risposta quando quella chiamata finisce e non entro i prossimi kilobyte, e l'avanzamento di
quel file arriva tutto insieme. Su Linux la copia è ancora fatta a pezzi, quindi là non vale
né l'una né l'altra cosa.

# Le somme di controllo

*Somme di controllo…* nel menu File scrive un elenco di impronte per quello che è selezionato:
SHA-256, SHA-1, MD5 o CRC32. Un elenco per tutto il gruppo — chiamato come la cartella, o come
qualunque altra cosa scrivi — oppure uno accanto a ciascun file. Il formato è quello che
scrivono `sha256sum` e compagni, così l'elenco può essere verificato da chiunque tu lo mandi con
lo strumento che ha già, e quello che ti mandano può essere verificato qui.

*Verifica contro un elenco* lavora sull'elenco sotto il cursore. Quale hash l'ha prodotto viene
ricavato dalle impronte che contiene e non dal suo nome, quindi un file chiamato `SHA256SUMS`,
`sums.txt` o `release.md5` funzionano tutti. Quello che non corrisponde viene nominato nel
rapporto **e lasciato selezionato nel pannello**, perché la cosa che fai dopo uno scaricamento
sbagliato è riscaricarlo o cancellarlo, e entrambe lavorano sulla selezione.

**Il CRC32 viene scritto come `.sfv`**, perché è l'unica forma in cui qualcosa lo legge: prima
il nome e dopo le otto cifre esadecimali, `;` per un commento, e `file.sfv` accanto a un file
singolo. È un controllo d'errore e non un hash — due file con lo stesso CRC32 si fanno apposta
senza fatica — quindi risponde a *è arrivato intero* e a nient'altro. Un `.sfv` che ti mandano
si verifica come qualunque altro elenco.

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

**Dentro gli archivi** è una casella, e resta spenta finché non la spunti: spuntata, la ricerca
entra in ogni `.zip`, `.7z` e `.tar.*` che incontra e guarda cosa c'è dentro — per nome, e per
testo se ne hai dato uno. Un risultato si legge allora `src.zip!/doc/readme.md`, che `Enter`,
`F3` e `F5` trattano come qualunque altro file. È più lenta, ed è per questo che è una casella:
aprire ogni contenitore di un disco intero è un'altra ricerca. Un file dentro un archivio più
grande di 64 MB non viene letto per il testo — viene contato come un problema, perché un
archivio va spacchettato per essere letto e quello non starebbe da nessuna parte sensata.

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

Parecchie cose sono zip senza dirlo, e `Enter` apre anche quelle: `.jar`, `.war`, `.ear`,
`.aar`, `.apk`, `.aab`, `.ipa`, `.whl`, `.nupkg`, `.vsix`, `.xpi` — i pacchetti che si aprono
per vedere cosa c'è dentro. Anche un `.docx` o un `.epub` è uno zip, e di proposito non sta in
quella lista: lì il programma che legge un file così è la risposta più utile, e `Enter` glielo
passa. Se invece vuoi vederci dentro, `Shift+F2` lo spacchetta comunque, e un archivio
rinominato si apre per i suoi byte una volta che sei già dentro un contenitore.

`Enter` apre anche l'archivio Unix — `.a` e `.ar` — e un pacchetto Debian, `.deb`, `.udeb` o
`.ddeb`, che *è* uno di quelli: dentro ci trovi `debian-binary`, un `control.tar.*` e un
`data.tar.*`, e `Enter` su uno dei due tar ci va dentro, così i file che il pacchetto
installerebbe sono a due tasti di distanza. Questi si leggono e non si scrivono: li costruiscono
`ar` e `dpkg-deb`, e un pacchetto scritto senza il suo file di controllo sembrerebbe un
pacchetto senza esserlo. Il dialogo di compressione quindi non offre il formato, e un nome come
`libfoo.a` scritto lì dentro viene rifiutato con una ragione invece che scritto a metà.

Si apre anche un archivio dentro un archivio, e uno su un server o in un bucket. Nessuno dei due
è un file in cui questo programma possa spostarsi, quindi viene prima copiato nella cartella
temporanea, una volta sola, e letto da lì — una pausa la prima volta e niente dopo, con il
percorso che continua a dire dov'è davvero il file:

    sftp://srv/backup/sito.zip!/interno.tar.gz!/etc/nginx.conf

`[arc] max_depth` dice quanti contenitori in profondità si può andare e `[arc] max_spill_mb`
quanto può essere grande un contenitore prima che la copia venga rifiutata invece che fatta;
tutti e due dicono quale limite li ha fermati. Una cosa che un archivio così non può fare è
essere scritto: la copia se ne va quando se ne va il programma, quindi una modifica sarebbe una
modifica che nessuno rivedrebbe mai più.

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

Un 7z che questo programma *scrive* mette ogni file in un blocco suo, a meno che non spunti
**Blocchi solidi** nel dialogo di `Shift+F1`. Un blocco per file è più grande di quello che
scrive 7-Zip per default, e qui è il default giusto: in un file manager l'atto comune è
leggere un file fuori da un archivio, e un archivio solido fa sì che questo significhi
decodificare tutto quello che gli sta davanti. Spuntato, i file condividono un flusso e
l'archivio è più piccolo; un blocco si chiude a 64 file o 128 MB, così leggerne uno non
decodifica mai un albero intero. La casella viene ricordata per il dialogo dopo, e solo un 7z
ci fa qualcosa — un tar è solido comunque e uno zip non può esserlo.

Un 7z cifrato — anche uno con la testata cifrata, dove non si vedono nemmeno i nomi — chiede la
sua password. La scrivi e l'archivio si apre: la password resta per tutto il tempo in cui il
programma gira, quindi l'elenco e ogni file che poi apri da quell'archivio la chiedono una
volta sola, e non viene scritta da nessuna parte. Una sbagliata la richiede. Uno zip cifrato con
AES lo dice invece di chiedere: leggerli vuole una parte della libreria zip che questa
compilazione lascia fuori.

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

* `Ctrl+A` su una voce imposta quello che un archivio conserva di lei: i permessi e la data di
  modifica. Tutti e due stanno nella testata della voce, quindi cambiarne uno riscrive
  l'archivio — la riga in grigio nel dialogo lo dice — e tutto il resto resta com'era. Le
  caselle si aprono su quello che l'elenco sa, quindi un dialogo che non tocchi non scrive
  niente.

Un archivio dentro un archivio prende tutto questo. Si legge da una copia, perché ogni lettore
qui dentro legge un file, quindi scriverci dentro vuol dire scrivere la copia e poi rimettere la
copia nell'archivio da cui è uscita — fino al file che sta sul disco, che è quello che cambia
davvero. Tre zip uno dentro l'altro sono tre archivi riscritti, e il job dice quando ha finito.

Quello che non c'è: `F7` non crea una cartella dentro un archivio — nessuno di questi formati
conserva una cartella vuota che valga la pena di tenere. E non si può scrivere dentro un
archivio che sta su un *server*: anche quello si legge da una copia, e niente rimetterebbe
indietro la copia.

# Riscrivere un file di testo

Il testo arriva da altrove: uno zip da una macchina Windows, un dump da un database, un file
che ti ha mandato qualcuno. *File → Riscrivi come…* prende la selezione e riscrive ogni file
con la code page e il fine riga che scegli. Cosa *sono ora* lo deduce file per file, dal
byte-order mark dove c'è e da come sono i byte dove non c'è — un `.txt` di solo ASCII è tutte
le code page insieme, quindi la prima riga del dialogo sta lì per dirglielo quando la deduzione
sbaglia.

Tre cose vengono rifiutate invece che fatte male: un file che non sembra testo (niente di buono
viene dal "convertire" un'immagine di disco), un file oltre i 64 MB (per decodificarlo va letto
tutto), e una code page di destinazione che non può contenere tutti i caratteri del file —
detto per nome, invece di scrivere un file pieno di punti di domanda. Un file che uscirebbe
esattamente come è entrato non viene riscritto affatto: cambierebbe la sua data di modifica e
direbbe a ogni backup che è cambiato. Quello che si scrive va accanto all'originale e gli viene
rinominato sopra, così un'interruzione lascia l'originale intero, e uno script che era
eseguibile resta eseguibile.

# Cosa è un file

`Ctrl+A` apre il dialogo degli attributi: quello che il file *è*, invece di quello che contiene.

Su Linux e macOS sono i permessi, come dodici caselle — proprietario, gruppo e tutti, ognuno con
lettura, scrittura ed esecuzione, più setuid, setgid e sticky — e la riga sopra di esse dice cosa
è il file ora, sia come `rw-r--r--` sia come `0644`. Sopra ancora c'è *Proprietario*, come
`utente:gruppo`, e su Linux e macOS è una riga in cui scrivere una risposta diversa: ciascuna
metà può essere un nome o un numero, e una metà lasciata vuota resta com'è — `:staff` cambia
il gruppo e nient'altro. Dare un file a qualcun altro richiede un privilegio che un utente
normale non ha, e il gruppo si può impostare solo a uno di cui fai parte; quello che il sistema
rifiuta viene riferito per nome, e il dialogo non ti chiede una password per farlo — `sudo` è
a una riga di distanza nella riga di comando sotto i pannelli, dove si vede a cosa serve. Su
Windows la riga è solo da leggere: la proprietà lì è una voce in un descrittore di sicurezza, e
il pannello delle proprietà della shell è il posto che chiede il privilegio come si deve. Su Windows sono
invece sola lettura, nascosto, sistema, archivio, temporaneo, non in linea e se l'indicizzatore
può leggerlo — e una riga in grigio dice cos'altro è il file, quando è compresso, cifrato,
sparso o un punto di reparse: quelli si impostano facendo la cosa, non spuntando una casella.
La sola lettura è su entrambi, perché su Unix sono i bit di scrittura.

Su più file le caselle su cui non sono d'accordo si aprono come `[?]`, e premere *OK* lascia
quei bit come ciascun file li ha: un dialogo su cento file non li mette d'accordo su qualcosa
che non ti è stato chiesto. `Spazio` porta una casella così attraverso *accesa*, *spenta* e di
nuovo `[?]`, così una premuta per sbaglio si può rimettere a non cambiare niente. Le caselle su
cui invece sono d'accordo sono spunte normali, e rispondono per tutti.

Sotto le caselle stanno gli orari — *Cambiato*, *Ultima lettura* e *Creato* — come
`2026-10-24 17:42:05`. Un orario lasciato in bianco viene lasciato stare; un orario scritto
viene impostato. *Orari in UTC* li legge e li scrive in quell'orologio invece che in quello di
questa macchina, che è come si confronta un file qui con la sua copia su un server di
compilazione. I tre pulsanti sotto riempiono le righe senza rispondere al dialogo: *Originale*
rimette quello che il file ha, *Adesso* scrive questo momento, *Vuoto* le svuota. Su Unix c'è
un quarto orario mostrato e mai offerto — il momento in cui è cambiato il record del file
stesso, che nessun sistema ha una chiamata per impostare — e la data di creazione è mostrata
ovunque e scritta solo su Windows, l'unica piattaforma che ha una chiamata per farlo. Con una cartella fra quello
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

Qualcosa di scritto che non è una risposta — una data che non è una data, un nome che qui
nessun utente o gruppo porta — lascia il dialogo aperto con quello che hai scritto ancora
dentro, e dice quale parola non ha saputo interpretare.

Quello che non c'è ancora: i bit avanzati di Windows, e il proprietario lì.

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
