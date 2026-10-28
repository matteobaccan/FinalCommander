# I server

`Alt+F1` e `Alt+F2` elencano le connessioni salvate accanto ai dischi. `n` scrive un URL:

    sftp://io@host/srv/www
    ftp://io@host/pub
    ftps://io@host/pub        lo stesso server con TLS
    webdavs://io@host/remote.php/dav/files/io/
    webdav://io@host:8080/share

Un pannello connesso è un pannello normale: i tasti sono gli stessi, una copia da o verso di
esso è un lavoro normale con una normale barra di avanzamento, e `F4` modifica un file remoto
portandolo qui e riscrivendolo quando salvi. La porta segue il protocollo quando l'URL non la
dice — 22 per SFTP, 21 per entrambe le grafie di FTP — e un server che offre `ftp` e `ftps`
sulla porta 21 è due connessioni, non una.

## Cosa FTP non può fare, e lo dice

SFTP è un filesystem; FTP è un protocollo di *trasferimento* di file, e la differenza si vede:

* **Niente link.** Niente in FTP ne crea uno, e un elenco solo a volte dice dove punta.
* **Niente su cui posizionarsi.** Il visualizzatore legge un file remoto scaricandolo prima in
  ogni caso, quindi questo si vede solo su un file molto grande, dove l'attesa è tutto il file
  invece della prima pagina.
* **Tempi e permessi sono estensioni.** `MFMT` imposta un orario e `SITE CHMOD` i permessi; la
  maggior parte dei server ha entrambi, e uno che non li ha lo dice con le proprie parole
  invece di perdere in silenzio la data di una copia.
* **Una data è affidabile solo da un server moderno.** `MLSD`, che è la prima cosa che viene
  chiesta, dà l'orario in UTC al secondo. Un server troppo vecchio per averlo risponde con il
  testo che stampa il suo `ls`, dove sui file recenti manca l'anno e il fuso orario non è mai
  detto — quindi quelle righe sono mostrate **senza** data. Una data sbagliata di un anno è
  peggio di nessuna data: ordinare per data, e "solo i file più nuovi di questi" nel dialogo
  di copia, le crederebbero entrambi.

Un trasferimento apre una connessione tutta sua e fa di nuovo il login, che è quello che fanno
i client FTP grafici e quello che evita che un elenco resti in attesa dietro a uno scaricamento.
Una password è quindi tenuta in memoria mentre la connessione è aperta — non viene scritta in
nessun posto — e a un utente chiamato `anonymous` o `ftp` non viene chiesta affatto.

FTPS è TLS esplicito: la connessione parte in chiaro e viene protetta con `AUTH TLS` prima che
la password parta, e anche le connessioni dati sono protette. Il certificato è verificato
contro le autorità di questa macchina, che è quello che fa funzionare un server dietro la CA
aziendale senza spegnere niente. L'FTPS implicito sulla porta 990 è deprecato e qui non c'è.

## WebDAV

`webdav://` è HTTP e `webdavs://` è HTTPS — le porte sono 80 e 443 quando l'URL non le dice —
e il percorso è quello che pubblica il server, che su Nextcloud e ownCloud è
`/remote.php/dav/files/<utente>/` e su un Apache con `mod_dav` è quello che dice l'`Alias`.

Un elenco è una richiesta, un file è un `GET`, scriverne uno è un `PUT` del file intero, una
cartella è un `MKCOL`, e cancellare una cartella porta via il suo albero in una sola richiesta.
Quello che questo costa:

* **Un file si scrive intero.** Non c'è niente da troncare e niente in cui scrivere nel mezzo,
  quindi `F4` su un file remoto lo salva mandandolo tutto — che è comunque quello che fa
  l'editor, dato che porta il file qui e lo riscrive.
* **Un orario non si può impostare e nemmeno i permessi.** In WebDAV la data di modifica di un
  file è del server — è una proprietà *viva* — e HTTP non ha bit di permesso. Una copia verso
  un pannello WebDAV dice quindi che non ha potuto portare la data, invece di dichiarare
  successo e lasciare il file datato adesso. (`Ctrl+A` non ha niente da offrire su un percorso
  così.)
* **Una rinomina non sostituisce niente.** `F6` su un nome già occupato viene rifiutato dal
  server invece di sovrascriverlo in silenzio.
* **Una password viene mandata a ogni richiesta**, come autenticazione HTTP basic sulla
  connessione — che è il motivo per cui `webdavs://` è quello da usare e `webdav://` è per un
  server su questa macchina o dietro una VPN. Una condivisione che non chiede login si
  raggiunge lasciando fuori l'utente dall'URL.

Quello che non c'è ancora: l'autenticazione digest (basic è quello che in pratica accettano
tutti i server su TLS), il locking e le proprietà di quota.

Una host key che il programma non ha mai visto lo fa fermare e la mostra prima che venga
mandato qualcosa, e la registra solo se glielo dici. Una password viene chiesta quando serve,
non viene mai mostrata e non viene mai scritta da nessuna parte.

`F7` nel gestore delle connessioni salva una sessione, `F4` ne modifica una, e `d` disconnette.
Quello che viene salvato è l'host, l'utente e il percorso — non la password.

## I tunnel

`F9 → Comandi → Apri un tunnel…` (o `net.tunnel` dalla palette dei comandi) inoltra una porta
sulla connessione su cui sta il pannello — la stessa cosa di `ssh -L`, senza un secondo ssh e
senza una seconda autenticazione:

    8080:db.interno:5432     la 8080 di questa macchina raggiunge db.interno:5432 dal server
    db.interno:5432          la stessa porta su entrambi i lati
    5432                     la 5432 del server

Qualunque cosa si colleghi alla porta locale viene portata sulla sessione ed esce dall'altro
capo come se fosse stata aperta là — un client di database, un browser, o il pannello `s3://`
di questo stesso programma puntato su un MinIO che nessuno fuori dalla sala macchine può
raggiungere.

La porta locale è aperta su **127.0.0.1**, non su tutte le interfacce: un tunnel è la rete del
server consegnata a questa macchina, e aprirlo ovunque la consegnerebbe al resto dell'ufficio.
`Tunnel…` nello stesso menu elenca quelli aperti con tre contatori — connessioni portate,
aperte ora, rifiutate dall'altro capo — e `Enter` su una riga lo chiude, liberando subito la
porta.

## I bucket

Un servizio di object storage — S3 stesso, o MinIO, Ceph, R2, Wasabi, qualunque cosa parli lo
stesso protocollo — è anch'esso un pannello, una volta che `fcmd.toml` ha un blocco `[[s3]]`
che dice dov'è:

    s3://logs/2024/08/
    s3://lavoro@logs/2024/08/     il servizio chiamato `lavoro`
    s3://lavoro@/                 i bucket che quel servizio vede

Un bucket non ha cartelle dentro. `2024/08/13/access.log` è una chiave con delle barre dentro,
e quello che il pannello mostra come cartella è l'insieme delle chiavi che cominciano allo
stesso modo fino alla barra successiva — cosa che calcola il servizio, non questo programma. È
per questo che un prefisso si apre in un bucket da dieci milioni di oggetti alla stessa
velocità che in uno da dieci.

Chiedere tutto quello che sta sotto un prefisso, a qualunque profondità, è un'altra cosa:
quello è davvero un elenco di tutto il bucket, mille richieste per un milione di chiavi, e si
ferma a `listing_cap` righe. Quando si ferma, il piede del pannello dice `(parziale)` per tutto
il tempo in cui quell'elenco resta sullo schermo, perché un conteggio che non è il conteggio
totale non deve sembrarlo.

Le chiavi non stanno mai in `fcmd.toml`. Vengono da `AWS_ACCESS_KEY_ID` e
`AWS_SECRET_ACCESS_KEY`, o da `~/.aws/credentials` — il file che gli strumenti AWS si dividono,
profili compresi. Un bucket che chiunque può leggere vuole `credentials = "anonymous"` e nessuna
chiave.

`F3` su un oggetto lo porta giù in una copia temporanea e mostra quella, come fa per un file su
un server, e `net.fetch_limit_mb` è il tetto a quello che scarica. `F4` modifica quella copia e
il salvataggio la rimanda su con una sola scrittura.

`F5`, `F6`, `F7` e `F8` funzionano su un bucket come funzionano altrove, con lo stesso dialogo,
la stessa barra di avanzamento, la stessa domanda sulla sovrascrittura e lo stesso `Esc`. Una
copia dentro un bucket è un caricamento preso dal disco mentre scorre; una copia fuori è uno
scaricamento scritto direttamente nel file; una copia da una chiave a un'altra dentro lo stesso
servizio la fa il servizio, quindi i byte non passano da qui, ed è questo che rende
sostenibile copiare un oggetto grande.

Alcune cose sono la natura dell'object storage, non lacune:

* **Non ci sono cartelle.** `a/b/c.txt` è una chiave, e la `a/` che un pannello mostra è
  ricavata dalle chiavi. `F7` scrive l'oggetto vuoto chiamato `a/b/` che usano tutte le console
  per rendere visibile un prefisso prima che ci sia qualcosa dentro, e la prima chiave vera
  sotto di esso rende quel segnaposto inutile.
* **Una rinomina è una copia e una cancellazione**, che il servizio fa da sé. Non è atomica: un
  errore fra le due lascia entrambe le chiavi, e il programma lo dice invece di nasconderlo.
* **Tempi e permessi non si conservano.** Un oggetto porta l'orario in cui il servizio l'ha
  scritto e nessun altro; l'accesso a un bucket è la sua policy, che non è un insieme di bit di
  permesso. Una copia dentro un bucket non conserva quindi la data di modifica del file, perché
  non c'è dove metterla.
* **Una scrittura è un oggetto.** Non si accoda, non si scrive nel mezzo e non si riprende — e
  non esiste nemmeno un oggetto scritto a metà, che è l'altra faccia della stessa medaglia. Un
  oggetto sopra i cinque gigabyte vuole un caricamento multipart, che qui non c'è ancora; è il
  rifiuto del servizio stesso a dirlo.
* **Un servizio configurato senza credenziali legge e nient'altro.** Le sue righe sono marcate
  in sola lettura, e i tasti che scriverebbero non vengono offerti.
