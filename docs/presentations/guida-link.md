# FinalCommander Link — guida

Il Link mostra in un pannello una cartella di un altro computer della **stessa rete locale**.
Basta FinalCommander sui due computer: niente server, niente account, niente da configurare
sul router. Nulla è raggiungibile finché chi sta all'altro computer non dice sì, oppure finché
chi chiama non dà il PIN giusto.

In breve:

| | Chi condivide | Chi chiama |
| --- | --- | --- |
| Con una persona davanti | *Comandi → Link: accetta chiamate sì/no* | sessione *Link* nel NetBox |
| Senza nessuno davanti | `fcmd --link-pin`, poi `fcmd --link-serve CARTELLA` | la stessa sessione, con il PIN |

---

## 1. Condividere una cartella

1. Porta un pannello sulla cartella da condividere. Deve essere una cartella di questo
   computer, non un server o un archivio.
2. Premi `F9` e scegli *Comandi → Link: accetta chiamate sì/no*.
3. La riga di stato mostra l'**ID** di questo computer, nove cifre come `123 456 789`, e la
   cartella condivisa. Nell'angolo in basso compare `LINK`.

La cartella è offerta **in sola lettura**: chi chiama può guardare e copiare, non cambiare.
Per smettere di condividere si sceglie di nuovo la stessa voce del menu.

L'ID non è un segreto: serve solo a trovare la macchina. Da solo non apre niente.

**A ogni avvio, da solo.** Nelle impostazioni (`fcmd.toml`):

```toml
[link]
accept = true
share = "~/Pubblica"
name = "PC dello studio"   # il nome che vede chi chiama; vuoto: il nome del computer
```

## 2. Chiamare l'altro computer

1. Apri il NetBox con `Alt+N` e premi `Ins` per una nuova sessione.
2. Come **protocollo** scegli *Link*.
3. Come **host** scrivi l'ID dell'altro computer, con o senza spazi.
4. L'**utente** lascialo vuoto. Prende la prima cartella che l'altro offre.
5. Salva con `F10`, oppure scegli *Connetti* per andarci subito.

FinalCommander cerca la macchina sulla rete locale e la chiama. Nella riga di stato compare un
**codice di sei cifre**, per esempio `calling 123456789: the code on its screen must be 042 917`.

## 3. Il codice di sei cifre

Sull'altro computer si apre un dialogo con:

- chi chiama (nome, ID, indirizzo);
- la cartella chiesta;
- **lo stesso codice di sei cifre**.

Chi sta all'altro computer confronta il codice con il tuo, a voce, al telefono o da una
scrivania all'altra, e accetta **solo se è uguale**. Un codice diverso vuol dire che qualcuno
si è messo in mezzo.

- Il pulsante già acceso è **Annulla**. Un `Invio` premuto per caso, magari mentre si
  scriveva un comando, rifiuta la chiamata. Per accettare ci si sposta su *Accetta* con
  `←` e si preme `Invio`.
- Se nessuno risponde entro un minuto, la chiamata è rifiutata.
- Una chiamata che arriva mentre si usa l'editor o un altro dialogo aspetta che tornino i
  pannelli.

Accettata la chiamata, il tuo pannello mostra la cartella dell'altro computer. Si usa come un
server SFTP: `F3` per leggere, `F5` per copiare qui, la ricerca e il confronto.

## 4. Durante il collegamento

Sul computer che condivide:

- l'angolo della riga di stato dice `LINK 1`, con il numero di chi sta leggendo;
- la riga di stato avvisa quando qualcuno entra e quando esce;
- `Ctrl+Alt+L` sui pannelli **chiude subito tutte le chiamate**;
- spegnere il Link dal menu le chiude anche.

## 5. Il PIN, per i propri computer

Per i propri computer il codice ogni volta è scomodo. Con un PIN si entra senza la domanda.

**Sul computer che condivide**, da un terminale:

```sh
fcmd --link-pin
```

Chiede il PIN due volte, senza mostrarlo. Deve avere almeno sei caratteri. Un PIN vuoto lo
toglie, e le chiamate tornano a chiedere alla persona.

**Sul computer che chiama**, nella sessione *Link* del NetBox scegli *ask* come
**autenticazione**. Quando apri la sessione viene chiesta la password: lì si scrive il PIN.

Il PIN:

- **non passa mai in rete**: i due computer dimostrano di conoscerlo senza dirlo, e chi
  intercetta il traffico non impara niente;
- non è salvato in chiaro: sul disco resta solo un valore derivato, nel file `link-pin`;
- dopo **cinque PIN sbagliati** dallo stesso indirizzo, quell'indirizzo è bloccato per
  **dieci minuti**.

Se chi chiama dà un PIN a un computer che non ne ha, la chiamata va alla persona come al
solito, con il codice di sei cifre.

## 6. Un computer senza nessuno davanti

Un router con una chiavetta USB, un Raspberry Pi o un server di casa possono condividere una
cartella senza schermo:

```sh
fcmd --link-pin                 # una volta
fcmd --link-serve /mnt/usb      # resta attivo finché non lo si ferma
```

Non essendoci nessuno a rispondere, **entra solo chi dà il PIN**: senza un PIN impostato
`--link-serve` non parte. Al posto della cartella sulla riga di comando si può usare
`link.share` nelle impostazioni.

All'avvio stampa l'ID, il nome e la porta. Poi scrive una riga per ogni chiamata:

```text
FinalCommander Link: ID 228 869 742 (nas), TCP port 41123, sharing /mnt/usb read only; callers need the PIN.
2026-11-25 15:07:06 refused, no PIN given: portatile (ID 442 063 104, 192.168.1.20)
2026-11-25 15:07:07 refused, wrong PIN: portatile (ID 442 063 104, 192.168.1.20)
2026-11-25 15:07:07 in, by its PIN: portatile (ID 442 063 104, 192.168.1.20)
2026-11-25 15:09:41 left: portatile (ID 442 063 104, 192.168.1.20)
```

### Come servizio su OpenWrt

1. Copia `fcmd` in `/usr/bin` (sul router, l'archivio `-compact-`).
2. Copia `packaging/openwrt/fcmd-link` in `/etc/init.d/` e rendilo eseguibile.
3. In cima al file correggi `FCMD`, `CONFIG` e `SHARE`.
4. Imposta il PIN: `fcmd --config /etc/fcmd/fcmd.toml --link-pin`.
5. Avvia: `/etc/init.d/fcmd-link enable && /etc/init.d/fcmd-link start`.
6. Il registro si legge con `logread -e fcmd`.

### Come servizio systemd (Raspberry Pi, Linux)

1. `fcmd --link-pin`, come l'utente che farà girare il servizio.
2. Copia `packaging/systemd/fcmd-link.service` in `~/.config/systemd/user/` e correggi la
   cartella nella riga `ExecStart`.
3. `systemctl --user enable --now fcmd-link`.
4. Il registro si legge con `journalctl --user -u fcmd-link`.

## 7. Firewall

Il Link usa due porte:

- **UDP 47831**, per farsi trovare;
- **una porta TCP** per le chiamate, scelta dal sistema e comunicata a chi cerca.

Se un firewall blocca le connessioni in entrata, fissa la porta TCP nelle impostazioni e apri
le due porte, solo verso la rete locale:

```toml
[link]
port = 47832
```

Il Link accetta solo indirizzi privati: reti di casa e d'ufficio (192.168.x.x, 10.x.x.x,
172.16–31.x.x) e gli indirizzi locali IPv6. Una chiamata da internet viene scartata senza
essere letta. Per raggiungere un computer lontano si usa SFTP.

## 8. Quando qualcosa non va

| Messaggio | Che cosa vuol dire | Che cosa fare |
| --- | --- | --- |
| *no FinalCommander with this ID answered on the local network* | nessuno ha risposto alla ricerca | controlla l'ID, che il Link sia acceso dall'altra parte, che siate sulla stessa rete e che la UDP 47831 non sia bloccata |
| *the call was not accepted* | dall'altra parte hanno rifiutato o non hanno risposto | richiama e fatti confermare il codice |
| *the machine's key has CHANGED* | l'altro computer ha una chiave diversa da quella vista l'ultima volta | se è stato reinstallato, togli la sua riga dal file `link-known` indicato nel messaggio; altrimenti non collegarti |
| *wrong PIN* | il PIN non è quello impostato | riprova; al quinto errore scatta il blocco |
| *too many wrong PINs from this machine* | cinque PIN sbagliati | aspetta i minuti indicati |
| *nobody is at this machine to answer* | è un `--link-serve` e non hai dato il PIN | nella sessione metti *ask* come autenticazione |
| *UDP port 47831 … is another FinalCommander here accepting calls?* | su questo computer un altro FinalCommander condivide già | ne basta uno: spegni l'altro |
| *Link shares a folder of this computer* | il pannello è su un server o un archivio | porta il pannello su una cartella locale |

## 9. Impostazioni e file

| Impostazione | Default | Significato |
| --- | --- | --- |
| `link.accept` | `false` | accetta chiamate all'avvio |
| `link.share` | vuoto | la cartella condivisa all'avvio e da `--link-serve` |
| `link.name` | vuoto | il nome che vede chi chiama; vuoto: il nome del computer |
| `link.port` | `0` | la porta TCP; 0: scelta dal sistema |

Accanto al file delle impostazioni (`fcmd --paths` dice dov'è):

| File | Contiene |
| --- | --- |
| `link-id` | l'ID di questo computer |
| `link-cert.der`, `link-key.der` | il certificato e la chiave di questo computer, la chiave leggibile solo dall'utente |
| `link-known` | le impronte dei computer chiamati, una riga ciascuno |
| `link-pin` | il valore derivato dal PIN, se c'è |

Questi file vengono creati solo la prima volta che si usa il Link.

## 10. Che cosa protegge, in breve

- **Tutto è cifrato** (TLS 1.3), con una chiave propria per ogni computer.
- **Nessuno entra senza un sì**: il codice confermato da una persona, oppure il PIN.
- **Sola lettura**, controllata dal computer che condivide, qualunque cosa mandi chi chiama.
- Non si esce dalla cartella condivisa: `..` e i collegamenti che portano fuori sono rifiutati.
- **Solo la rete locale.**
- Un computer la cui chiave cambia viene rifiutato.
