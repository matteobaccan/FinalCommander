key-help = Hilfe
key-menu = BenuMn
key-view = Ansehen
key-edit = Bearb
key-copy = Kopier
key-move = Verschb
key-mkdir = VerzEr
key-delete = Löschen
key-pulldown = KonfMn
key-quit = Ende
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Hinzu
key-extract = Extrah
key-arccmd = ArcBef
key-edit-new = Bearb…
key-rename = Umben
key-group = Gruppe
key-selup = MarkOb
key-left = Links
key-right = Rechts
key-sort-name = Name
key-sort-ext = Erweit
key-sort-write = Schreib
key-sort-size = Größe
key-sort-unsort = Unsort
key-sort-create = Erstel
key-sort-access = Zugrif
key-sort = Sortrn
key-mklink = Verknü
key-find = Suchen
key-history = Verlau
key-folders = OrdnVe
key-plugins = Plugin

list-up = Hoch
list-folder = Ordner
col-name = Name
col-ext = Erw
col-size = Größe
col-allocated = Belegt
col-modified = Geändert
col-created = Erstellt
col-accessed = Zugriff
col-attr = Attr
col-owner = Besitzer
col-group = Gruppe
col-descr = Beschreibung
col-links = Lnk
col-git = Git
descr-title = Beschreibung von { $name }
descr-set = { $name } beschrieben
descr-cleared = Beschreibung von { $name } entfernt
descr-not-here = Beschreibungen liegen neben einem Verzeichnis auf diesem Rechner
col-date-time = Datum      Zeit
panel-list-count = { $count } Einträge in der Liste
panel-empty = (leer)
panel-loading = lese…

prompt-select-mask = Dateien nach Maske auswählen:
prompt-deselect-mask = Auswahl nach Maske aufheben:
prompt-goto = Wechseln zu Verzeichnis:
prompt-filter = Maske oder Teil des Namens:
prompt-search = Schnellsuche:

dialog-error = Fehler
dialog-help-hint = Esc zurück zum Inhalt · F7 sucht in allen Kapiteln · F1 schließt · ↑↓ Bild↑ Bild↓ zum Blättern
dialog-help-index = FinalCommander { $version } — Hilfe
dialog-help-hint-index = Eingabe öffnet ein Kapitel · F7 sucht in allen Kapiteln · Esc schließt
help-search-prompt = Hilfe durchsuchen
help-search-none = Keine Zeile der Hilfe enthält „{ $query }“
help-search-title = „{ $query }“ in der Hilfe: { $count } Zeilen
output-search-prompt = In der Ausgabe suchen
output-search-empty = Es wurde noch nichts ausgegeben
output-search-found = Zeile { $line } von { $lines }
output-search-wrapped = Zeile { $line } von { $lines }, wieder ab der neuesten
output-search-none = Keine Zeile der Ausgabe enthält „{ $query }“

msg-not-implemented = { $command } ist in dieser Version noch nicht umgesetzt
msg-selected = { $count } Einträge ausgewählt
msg-deselected = { $count } Einträge abgewählt
msg-mask-invalid = ungültige Maske: { $error }
msg-theme = Thema: { $name }
msg-font-bigger = Schrift: eine größere beim Terminal angefragt
msg-font-smaller = Schrift: eine kleinere beim Terminal angefragt
msg-font-reset = Schrift: die anfängliche beim Terminal angefragt
msg-sort = Sortierung: { $key }{ $reverse }
msg-sort-groups-on = Sortiergruppen ein ({ $count })
msg-sort-groups-off = Sortiergruppen aus ({ $count })
msg-sort-groups-none = keine Sortiergruppen festgelegt: siehe [[sort_group]] in der Konfiguration
msg-view = Ansicht: { $mode }
msg-hidden-on = versteckte Dateien werden gezeigt
msg-hidden-off = versteckte Dateien sind verborgen
msg-no-history = keine weitere Historie
msg-tab-last = ein Fenster hat immer einen Reiter
msg-tab-end = der Tab ist schon an diesem Ende der Reihe
tabs-title = Tabs
msg-tab-moved = Reiter in das andere Fenster verschoben
msg-tab-locked = dieser Reiter ist auf sein Verzeichnis festgelegt
msg-tab-locked-on = Reiter auf sein Verzeichnis festgelegt
msg-tab-locked-off = Reiter freigegeben
msg-bookmark-set = Verknüpfung { $digit }: { $path }
msg-bookmark-unset = Verknüpfung { $digit } ist nicht gesetzt
msg-bookmark-added = { $path } gemerkt
msg-bookmark-removed = { $path } entfernt
msg-bookmark-known = dieser Ordner ist schon in der Liste
msg-dirsize = { $path }: { $bytes } in { $files } Dateien
msg-dirsize-many = { $count } Ordner werden gemessen …
usage-title = Speicherbelegung: { $path } · { $size }
usage-not-here = die Belegung eines Ordners wird auf einem Datenträger gemessen, nicht in einem Archiv oder einer Fundliste
msg-dirsize-no-folder = hier gibt es keinen Ordner zu messen

prompt-copy-to = { $what } kopieren nach:
prompt-move-to = { $what } verschieben/umbenennen nach:
prompt-mkdir = Verzeichnis anlegen:
prompt-tab-name = Name des Reiters:
prompt-bookmark-name = Name des Ordners:
what-selected = { $count } ausgewählte Einträge
confirm-delete-title = Löschen
confirm-delete-body =
    { $count ->
        [one] { $name } ({ $bytes }) löschen?
       *[other] { $count } Einträge ({ $bytes }) löschen?
    }
prompt-link = Verknüpfung auf { $name } erstellen als:
link-created = { $name } erstellt
confirm-wipe-title = Sicheres Löschen
confirm-wipe-body =
    { $count ->
        [one] { $name } ({ $bytes }) überschreiben und löschen?
       *[other] { $count } Einträge ({ $bytes }) überschreiben und löschen?
    }
confirm-wipe-honest = Die Bytes werden einmal überschrieben, dann wird die Datei entfernt. Auf einer SSD, einem Copy-on-Write-Dateisystem oder mit Snapshots können ältere Kopien überleben — und der Name bleibt im Verzeichnis und im Journal.
job-copy = Kopieren
job-move = Verschieben
job-delete = Löschen
job-wipe = Sicheres Löschen
job-trash = In den Papierkorb
job-replace = Ersetzen
job-pack = Packen
job-rebuild = Archiv wird geschrieben

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Dateiattribute
attr-count = { $count } ausgewählte Einträge
attr-now = jetzt { $text } ({ $octal })
attr-owner-read = Eigentümer darf lesen
attr-owner-write = Eigentümer darf schreiben
attr-owner-exec = Eigentümer darf ausführen
attr-group-read = Gruppe darf lesen
attr-group-write = Gruppe darf schreiben
attr-group-exec = Gruppe darf ausführen
attr-other-read = Alle dürfen lesen
attr-other-write = Alle dürfen schreiben
attr-other-exec = Alle dürfen ausführen
attr-setuid = Als Eigentümer ausführen (setuid)
attr-setgid = Als Gruppe ausführen (setgid)
attr-sticky = Nur der Eigentümer darf löschen (sticky)
attr-readonly = Nur lesen
attr-hidden = Versteckt
attr-system = System
attr-archive = Archiv
attr-temporary = Temporär
attr-offline = Offline
attr-indexed = Der Indexdienst darf hineinsehen
attr-compressed = komprimiert
attr-encrypted = verschlüsselt
attr-sparse = dünn besetzt
attr-reparse = ein Reparse-Punkt
attr-also = außerdem { $what }
attr-mtime = Geändert
attr-atime = Zuletzt gelesen
attr-btime = Erstellt
attr-ctime = Eintrag geändert
attr-btime-readonly = (die Erstellzeit lässt sich nur unter Windows setzen)
attr-utc = Zeiten in UTC
attr-owner = Eigentümer (Benutzer:Gruppe)
attr-owner-is = Eigentümer { $who }
btn-attr-original = Original
btn-attr-current = Jetzt
btn-attr-blank = Leer
attr-recursive = Und alles in den Ordnern
attr-bad-time = `{ $text }` ist kein Datum mit Uhrzeit (JJJJ-MM-TT HH:MM:SS)
attr-bad-owner = hier heißt kein Benutzer und keine Gruppe `{ $text }`
attr-in-archive = eine Änderung schreibt das Archiv neu, mit allen anderen Einträgen wie sie sind
job-attrs = Attribute werden gesetzt

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Kopieren
copy-title-move = Umbenennen oder verschieben
copy-target = Nach
copy-conflict = Wenn es schon da ist
copy-conflict-ask = fragen
copy-conflict-overwrite = überschreiben
copy-conflict-newer = überschreiben, wenn die Quelle neuer ist
copy-conflict-skip = überspringen
copy-conflict-rename = der Kopie einen freien Namen geben
copy-conflict-append = hinten anhängen
copy-only-newer = Nur Dateien, die neuer sind als die dort
copy-filter = Nur die passenden Namen
copy-follow-links = Das Ziel des Verweises kopieren, nicht den Verweis
copy-keep-times = Zeiten behalten
copy-keep-mode = Rechte behalten
copy-verify = Jede Datei zurücklesen und vergleichen
copy-several = { $count } Ziele, eines nach dem anderen

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = ein Terminal mit { $shell }
term-hint = Ctrl+Alt+E: zurück zu den Panels
term-ended = beendet — eine Taste schließt
job-mkdir = Verzeichnis anlegen
job-planning = planen…
job-counters = { $files } / { $filestotal } Dateien · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · { $elapsed } vergangen · { $eta } übrig
job-rates-no-eta = { $speed }/s · { $elapsed } vergangen
job-hint = Esc bricht ab
job-hint-queue = Esc bricht ab · B Hintergrund
job-queued = { $kind } steht in der Warteschlange, mit { $ahead } weiteren unfertigen Jobs · Alt+Shift+F5 listet sie
job-background = { $kind } läuft im Hintergrund weiter · Alt+Shift+F5 listet die Jobs
job-asks = { $kind } wartet auf eine Antwort · Alt+Shift+F5
job-paused = { $kind } angehalten
job-resumed = { $kind } läuft weiter
job-retry-nothing = Nichts zu wiederholen
job-still-waiting = Dieser Job wartet, bis er dran ist
jobs-title = Jobs ({ $active } nicht fertig)
jobs-hint = Enter zeigen · Leertaste Pause · Entf stoppen · F5 wiederholen · Ctrl+↑↓ Reihenfolge · Esc zu
jobs-quit-title = Es laufen noch Jobs
jobs-quit-body = Diese Jobs werden gestoppt, wenn das Programm jetzt endet:
job-state-running = läuft
job-state-paused = angehalten
job-state-asking = fragt
job-state-queued = wartet
job-state-held = zurückgehalten
job-state-done = fertig
job-state-failed = Fehler
job-state-cancelled = abgebrochen
job-done = { $kind }: { $files } Dateien, { $bytes }
job-done-renamed = { $kind }: { $files } Dateien ohne Kopieren verschoben
job-done-skipped = , { $skipped } übersprungen
job-done-verified = , { $verified } geprüft
job-done-unverified = , { $unverified } hier nicht zurücklesbar
job-done-errors = , { $errors } Fehler
job-cancelled = { $kind } abgebrochen
job-errors-title = Fehler
conflict-title = Das Ziel existiert
conflict-dst = Ziel     { $path }   { $bytes }
conflict-src = Quelle   { $path }   { $bytes }
conflict-newer = die Quelle ist neuer
conflict-older = das Ziel ist neuer
btn-overwrite = Überschreiben
btn-overwrite-all = Alle
btn-skip = Überspringen
btn-skip-all = Alle überspringen
btn-retry = Wiederholen
btn-move = Verschieben
btn-delete = Löschen
btn-wipe = Überschreiben
btn-copy = Kopieren
btn-discard = Verwerfen
btn-kill = Beenden
btn-quit = Beenden
btn-play = Abspielen
btn-run = Ausführen
btn-accept = Annehmen
btn-cancel = Abbrechen
msg-nothing-to-do = nichts ausgewählt
cmd-complete-nothing = nichts, womit sich das ergänzen ließe
complete-several = { $count } Namen beginnen so
form-moving = die Pfeile verschieben den Dialog · Enter oder Esc lässt ihn dort
form-moved = der Dialog bleibt, wo Sie ihn hingestellt haben
msg-trash-refused = Der Papierkorb nimmt es nicht — Shift+Entf löscht endgültig.

# Checksums (SPEC 03 §9)
sum-title = Prüfsummen
sum-algorithm = Hash
sum-one-file = Eine Liste für alle
sum-name = Name
sum-working = { $name }: { $done } von { $total }
sum-verifying = Prüfung gegen { $name }
sum-stopped = abgebrochen
sum-written = { $name } geschrieben
sum-no-name = die Liste braucht einen Namen
sum-local-only = eine Prüfsumme kann nur eine Datei auf diesem Rechner lesen
sum-all-match = alle { $count } stimmen
sum-verify-title = Prüfsummen
sum-verify-summary = { $ok } stimmen, { $bad } nicht
sum-differs = stimmt nicht
sum-missing = nicht vorhanden
sum-unreadable = nicht lesbar

# Synchronising two directories (SPEC 03 §10)
sync-title = Abgleichen
sync-between = { $left }  ↔  { $right }
sync-recursive = Auch alles in den Unterverzeichnissen
sync-both-ways = In beide Richtungen, das Neuere gewinnt
sync-delete-extra = Anbieten zu entfernen, was die andere Seite nicht hat
sync-by-content = Die Bytes vergleichen, nicht Größe und Zeit
sync-filter = Nur die passenden Namen
sync-walking = prüft { $at } — bisher { $pairs }
sync-planned = { $copies } zu kopieren ({ $bytes }), { $deletes } zu entfernen
sync-hint = Leer nächste Antwort · ← → Richtung · = lassen · F2 ausführen · Esc abbrechen
sync-hint-walking = Esc bricht die Suche ab
sync-nothing-to-do = jede Zeile sagt, alles so zu lassen
sync-local-only = ein Abgleich kann nur auf diesem Rechner lesen und schreiben
sync-same-directory = beide Fenster sind im gleichen Verzeichnis

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nichts sagt, wie { $name } zu öffnen ist
usermenu-empty = Noch kein Benutzermenü: schreibe es in { $path }
usermenu-last-gone = der zuletzt gewählte Eintrag ist nicht mehr in diesem Menü
usermenu-bad-condition = Bedingung nicht verstanden — { $problem }
assoc-desktop = An den Desktop übergeben
assoc-open-with = { $name } öffnen mit
assoc-no-pager = keine Zuordnung und kein $PAGER sagen, womit das zu lesen ist
assoc-no-editor = keine Zuordnung und kein $VISUAL sagen, womit das zu schreiben ist

key-wrap = Umbruch
key-hex = Hex
key-text = Text
key-dump = Dump
key-doc = Doku
key-search = Suchen
key-code = Kodier
key-follow = Folgen
viewer-prompt-search = Suchen ({ $mode }):
viewer-prompt-goto = Zu Offset springen, oder NN% :
viewer-not-found = `{ $query }` nicht gefunden
viewer-follow-on = folgt dem Dateiende
viewer-follow-off = Folgen aus
viewer-codepage = Kodierung: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = keine frühere Position
viewer-empty = (leere Datei)
viewer-not-markdown = keine Markdown-Datei, oder zu groß zum Setzen
viewer-not-table = keine Tabellendatei, oder zu groß, um sie ganz zu lesen
viewer-base64-done = Dekodiert: { $bytes }
viewer-not-base64 = Diese Datei ist kein Base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disass
viewer-disasm-failed = { $name } kann nicht disassembliert werden: { $error }
viewer-decompile-running = { $tool } läuft auf { $name }…
viewer-decompile-none = hier kann nichts { $format } dekompilieren; Werkzeug installieren und eine [[decompile]]-Regel in fcmd.toml ergänzen
arc-needs-program = dieses Archiv liest `{ $program }`, das auf diesem Rechner fehlt
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame gilt der Datei selbst: zuerst zu ihr zurückkehren
viewer-derived = { $producer } von { $name }

key-save = Speich
key-replace = Ersetz
editor-prompt-goto = Zu Zeile, oder Zeile:Spalte :
editor-prompt-save-as = Speichern unter:
editor-saved = { $name }: gespeichert, { $bytes } Bytes
editor-nothing-to-save = { $name } hat keine Änderungen
editor-not-found = `{ $query }` nicht gefunden
editor-replaced = { $count } Ersetzungen
editor-read-only = { $name } ist schreibgeschützt
editor-readonly-on = schreibgeschützt: Änderungen werden abgelehnt
editor-readonly-off = Schreibschutz aus
editor-no-undo = nichts mehr zurückzunehmen
editor-no-redo = nichts wiederherzustellen
editor-discard-title = Ungespeicherte Änderungen
editor-discard-body = { $name } hat ungespeicherte Änderungen. Verlassen und verwerfen?
editor-changed-title = Auf der Platte geändert
editor-changed-body = { $name } wurde seit dem Öffnen geändert. Überschreiben?
editor-reloaded = { $name } neu geladen
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Verbindungen
net-sites-hint = Eingabe verbinden · n URL · Einfg neu · F4 ändern · d trennen · Esc schließen
net-no-sites = Noch keine Sites. `n` für eine URL, oder [[site]]-Blöcke in die Konfiguration.
net-not-a-stored-session = diese Zeile ist eine offene Verbindung, keine gespeicherte Sitzung
net-site-forgotten = { $name } ist nicht mehr gespeichert
net-prompt-url = Verbinden mit:
net-prompt-password = Passwort für { $user }@{ $host }:
net-prompt-passphrase = Passphrase für den Schlüssel von { $user }@{ $host }:
net-connecting = verbinde mit { $host }…
net-connected = mit { $host } verbunden
net-disconnected = { $host } getrennt
net-not-connected = { $host } ist nicht verbunden
net-local-panel = dieses Panel liegt auf einem lokalen Dateisystem
net-nothing-pending = keine Verbindung wartet darauf
net-hostkey-title = Unbekannter Hostschlüssel
net-hostkey-body = { $host } steht nicht in known_hosts. Fingerabdruck:
net-hostkey-question = Eintragen und verbinden?
net-hostkey-changed-title = Der Hostschlüssel hat sich GEÄNDERT
net-hostkey-changed-body = { $host } antwortete mit einem anderen als dem gespeicherten Schlüssel.
net-hostkey-changed-where = Der gespeicherte steht in { $path }, Zeile { $line }.
net-hostkey-changed-what = Ein neu aufgesetzter Server sieht genauso aus wie ein Angreifer dazwischen. Hier wird nichts akzeptiert: Datei öffnen, Zeile lesen und selbst entfernen, wenn der Grund bekannt ist.
btn-open-file = Datei öffnen
net-fetching = hole { $name }…
net-viewing-copy = { $name }: lokale Kopie, für den Betrachter geholt
net-too-big = { $name } ist größer als { $limit }: net.fetch_limit_mb erhöhen
net-editing = { $name }: eine lokale Kopie wird bearbeitet; Speichern schreibt sie zurück
net-writing-back = { $name } wird zurückgeschrieben…
net-written-back = { $name }: zurückgeschrieben, { $bytes } Bytes
net-write-back-failed = Zurückschreiben fehlgeschlagen: { $error } — Ihre Änderungen liegen in { $local }
net-changed-title = Auf dem Server geändert
net-changed-body = { $name } wurde auf dem Server geändert, seit es geholt wurde. Überschreiben?
confirm-delete-permanent = Das löscht endgültig: für diese gibt es keinen Papierkorb.
confirm-trash-title = In den Papierkorb
confirm-trash-body =
    { $count ->
        [one] { $name } ({ $bytes }) in den Papierkorb verschieben?
       *[other] { $count } Objekte ({ $bytes }) in den Papierkorb verschieben?
    }
confirm-trash-where = Sie landen dort, wo der Desktop gelöschte Dateien aufbewahrt, und lassen sich von dort zurückholen.

# ---------------------------------------------------------------- Menüleiste (SPEC 10)
menu-left = &Links
menu-right = &Rechts
menu-files = &Dateien
menu-commands = &Befehle
menu-options = &Optionen
menu-brief = &Kurz
menu-medium = &Mittel
menu-full = &Voll
menu-detailed = Ausführ&lich
menu-owners = B&esitzer
menu-links = &Dateiverknüpfungen
menu-alt-full = Voll, Größen &ausgeschrieben
menu-descriptions = &Beschreibungen
menu-long-descriptions = Lan&ge Beschreibungen
menu-wide = B&reit
menu-info-panel = &Infofenster
menu-quick-view = Sc&hnellansicht
menu-tree-panel = Baumfens&ter
menu-tab-new = Neuer Reiter
menu-tab-close = Reiter schlie&ßen
menu-tab-next = N&ächster Reiter
menu-tab-prev = V&origer Reiter
menu-tab-move = Reiter in das andere Fenster
menu-tab-lock = Reiter auf sein Ver&zeichnis festlegen
menu-tab-name = Reiter benennen
menu-tab-list = Alle Reiter
menu-sort-modes = &Sortierung
menu-hidden = Versteckte und S&ystemdateien
menu-filter = Datei&filter
menu-filter-sets = Benannte Filter
menu-panel-onoff = Fe&nster ein/aus
menu-reread = Ne&u lesen
menu-change-drive = Laufwerk we&chseln
menu-view = &Ansehen
menu-edit = B&earbeiten
menu-edit-new = &Neue Datei bearbeiten
menu-copy = &Kopieren
menu-rename = Umbenennen ode&r verschieben
menu-link = &Verknüpfung erstellen
menu-multirename = Mehrere um&benennen…
menu-multirename-undo = Letzte Mehrfach-Umbenennung rückgängig
menu-mkdir = &Ordner erstellen
menu-delete = &Löschen
menu-wipe = &Sicher löschen
menu-checksum = &Prüfsummen…
menu-checksum-verify = Gegen eine Liste prü&fen
menu-split = Datei teilen…
menu-combine = Dateien zusammenfügen
menu-delete-permanent = Endg&ültig löschen
menu-archive-add = Zum Archiv &hinzufügen
menu-archive-extract = &Dateien entpacken
menu-archive-commands = Ar&chivbefehle
menu-archive-test = Archiv testen
menu-attributes = Da&teiattribute
menu-apply-command = Befehl an&wenden
menu-replace-in-files = In Dateien erset&zen…
menu-select-group = &Gruppe auswählen
menu-select-ext = Nach dieser Endung ausw&ählen
menu-unselect-ext = Nach dieser Endung abwählen
menu-unselect-group = Gr&uppe abwählen
menu-invert-selection = Auswahl u&mkehren
menu-select-all = Alles auswählen
menu-restore-selection = Auswahl w&iederherstellen
menu-dirsize = Ordnergr&öße
menu-find-file = &Datei suchen
menu-history = Befe&hlsverlauf
menu-view-history = Verlauf von Ansich&t und Bearbeitung
menu-folders-history = &Ordnerverlauf
menu-usage = S&peicherbelegung
menu-jobs = &Jobs
menu-branch = Zweigansicht
menu-bookmarks = Ordner-Verknüpfungen
menu-swap-panels = Fenster tau&schen
menu-sync-panel = Anderes &Fenster abgleichen
menu-panels-onoff = Fenster &ein/aus
menu-compare-folders = Ordner verglei&chen
menu-sync-folders = Ordner ab&gleichen
menu-compare-contents = Nach &Inhalt vergleichen
menu-panelize = Ausgabe eines Befehls als Panel
menu-view-filtered = Durch einen Befehl ansehen
menu-learn-keys = Tasten lernen
menu-tunnel = Tunnel &öffnen…
menu-tunnels = Tunnel…
tunnel-title = Tunnel: [lokal:]Host:Port
tunnel-list-title = Tunnel
tunnel-none = kein Tunnel offen
tunnel-open = Tunnel offen: { $what }
tunnel-closed = Tunnel geschlossen
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = ein Tunnel läuft über eine Verbindung: mit Alt+N eine öffnen
menu-connections = Verbindungen
menu-disconnect = Verbindung trennen
menu-user-menu = Benutzer&menü
menu-associations = D&ateizuordnungen
menu-folder-shortcuts = Ordner-&Kurzwahlen
menu-plugin-commands = Pl&ugin-Befehle
menu-screens-list = Fe&nsterliste
menu-macro-record = Makro auf&zeichnen
menu-macro-list = Makros
menu-task-list = Aufgaben&liste
menu-system-settings = Systemeinstellun&gen
menu-panel-settings = &Fenstereinstellungen
menu-interface-settings = Oberflächene&instellungen
menu-languages = Sprac&hen
menu-themes = Fa&rbschemata
menu-confirmations = Bestätigu&ngen
menu-viewer-settings = Eins&tellungen der Ansicht
menu-editor-settings = &Einstellungen des Editors
menu-codepages = &Zeichensätze
menu-sort-groups = S&ortiergruppen
menu-mask-groups = Datei&masken
menu-settings-all = Er&weiterte Einstellungen
menu-highlighting = &Dateihervorhebung
menu-plugins-config = Plugin-Konfig&uration
menu-help = Hi&lfe
menu-save-setup = Einstellungen &speichern
menu-quit = Beenden
opt-natural-sort = Natürliche Zahlenfolge
opt-allow-reverse = Umgekehrte Sortierung erlauben
opt-select-folders = Masken wählen auch Ordner aus
cmd-no-network-name = Ein Pfad auf dieser Platte hat keinen Netzwerknamen
opt-trash = Systempapierkorb verwenden
opt-system-copy = Kopierfunktion des Systems verwenden
opt-save-history = Verlauf zwischen Sitzungen behalten
opt-history-lifetime = Lebensdauer des Verlaufs, in Tagen
opt-forever = für immer
opt-fetch-limit = Größenbegrenzung für entfernte Dateien
opt-keepalive = SSH-Keep-alive
opt-off = aus
opt-show-hidden = Versteckte und Systemdateien anzeigen
opt-column-titles = Spaltentitel anzeigen
opt-status-line = Statuszeile anzeigen
opt-scrollbar = Scrollbalken anzeigen
opt-free-space = Freien Speicher anzeigen
opt-auto-refresh = Panel automatisch aktualisieren
opt-dirs-first = Ordner zuerst
opt-selected-first = Ausgewählte Dateien zuerst
opt-reverse = Umgekehrte Reihenfolge
opt-view = Fensteransicht
opt-sort = Sortieren nach
opt-quick-search = Schnellsuche
opt-quick-alt = mit Alt
opt-quick-plain = mit Buchstaben
opt-key-bar = Tastenleiste anzeigen
opt-status-bar = Statusleiste anzeigen
opt-syntax = Erkannte Sprachen einfärben
opt-markdown = Markdown lesefertig gesetzt
opt-table = Eine .csv oder .jsonl als Tabelle öffnen
opt-keymap = Tastenbelegung
opt-clock = Uhr in der Ecke
opt-mouse = Mausunterstützung
opt-clipboard = Ablage über das Terminal (OSC 52)
opt-command-screen = Die Ausgabe eines Befehls zeigen, während er läuft
opt-menu-always = Menüleiste immer anzeigen
opt-save-exit = Einstellungen beim Beenden speichern
opt-confirm-delete = Löschen
opt-confirm-overwrite = Überschreiben
opt-confirm-exit = Beenden
opt-confirm-editor = Editor mit ungespeicherten Änderungen verlassen
opt-applied = Einstellungen übernommen
opt-saved = gespeichert in { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Keine Ja-Nein-Einstellung: Enter gibt einen Wert ein
opt-settings-hint = ↑↓ wählen · tippen filtert · Enter oder F4 Wert · Leer umschalten · Entf Standard · Esc schließen
opt-language-set = Sprache: { $name }
sort-name = Name
sort-ext = Erweiterung
sort-size = Größe
sort-mtime = Datum
sort-btime = Erstellt
sort-atime = Zugriff
sort-descr = Beschreibung
sort-owner = Besitzer
sort-git = Git-Status
key-sort-descr = Beschr
key-sort-owner = Besitz
sort-none = Unsortiert
sort-set = Sortierung: { $name }
btn-ok = OK
pick-hint = ↑↓ wählen · tippen filtert · Enter übernehmen · Esc schließen
pick-empty = (nichts zur Auswahl)
# ------------------------------------------------- die drei Verläufe (SPEC 06 §6)
hist-pinned = { $line } ist angeheftet · nichts verdrängt sie mehr
hist-unpinned = { $line } ist nicht mehr angeheftet
hist-is-pinned = { $line } ist angeheftet · erst Einfg löst sie
hist-clear-body = Diesen Verlauf von allem leeren, was nicht angeheftet ist?
hist-cleared = der Verlauf ist leer bis auf das Angeheftete
history-hint = ↑↓ wählen · Enter übernehmen · Einfg anheften · Umschalt+Entf eine · Entf alle · Esc schließen


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Neue Verbindung
site-edit = Verbindung ändern
site-name = Name
site-host = Host
site-port = Port
site-user = Benutzer
link-title = Verknüpfung erstellen
link-name = Die Verknüpfung
link-kind = Art
link-kind-relative = symbolisch, relativ — übersteht das Verschieben des Baums
link-kind-absolute = symbolisch, absolut — übersteht das Verschieben der Verknüpfung
link-kind-hard = hart — ein zweiter Name für dieselbe Datei
menu-colors = F&arben…
palette-title = Farben von { $theme }
palette-hint = Eingabe Farbe · F2 speichern · Esc schließen
palette-ask = Eine Farbe für { $key }
palette-bad-color = { $value } ist keine Farbe (ein Name, 0–255 oder #rrggbb)
palette-saved = die Farben wurden in das Thema { $theme } geschrieben
palette-unsaved = die Farben sind nur auf dem Bildschirm geändert — F2 schreibt sie
opt-viewer-status-line = Oberste Zeile anzeigen (Name und Position)
opt-viewer-scrollbar = Scrollbalken anzeigen
opt-viewer-key-bar = Tastenleiste anzeigen
opt-viewer-wrap = Lange Zeilen
opt-viewer-wrap-off = nicht umbrechen
opt-viewer-wrap-char = am Rand umbrechen
opt-viewer-wrap-word = am Wort umbrechen
opt-viewer-tab-width = Tabulatorbreite
opt-viewer-remember = Position und Codepage jeder Datei merken
opt-editor-tabs = Tabulatoren
opt-editor-tabs-keep = unverändert lassen
opt-editor-tabs-typed = neu getippte umwandeln
opt-editor-tabs-all = beim Öffnen alle umwandeln
opt-editor-tab-width = Tabulatorbreite
opt-editor-auto-indent = Einzug der Zeile darüber übernehmen
opt-editor-whitespace = Leerzeichen und Tabulatoren zeigen
opt-editor-cursor-after = Der Cursor landet hinter dem Gefundenen
opt-editor-beyond-eol = Der Cursor darf über das Zeilenende hinaus
opt-editor-trim-trailing = Leerzeichen am Zeilenende beim Speichern entfernen
editor-not-a-character = diese Taste hat kein Zeichen zum Einfügen
paste-one-line =
    { $count ->
        [one] nur die erste Zeile eingefügt · eine weitere Zeile weggelassen
       *[other] nur die erste Zeile eingefügt · { $count } weitere Zeilen weggelassen
    }
clip-names =
    { $count ->
        [one] ein Name in der Ablage
       *[other] { $count } Namen in der Ablage
    }
clip-paths =
    { $count ->
        [one] ein Pfad in der Ablage
       *[other] { $count } Pfade in der Ablage
    }
clip-line = die Befehlszeile liegt in der Ablage
clip-selection = das Markierte liegt in der Ablage
clip-off = die Ablage ist abgeschaltet — `ui.clipboard` in den Einstellungen
history-gone = { $name } ist nicht mehr da
hl-new = Neue Farbgruppe
hl-edit = Farbgruppe
# ------------------------------------------------- die Unterschiede (SPEC 05 §3)
diff-title = Unterschiede
menu-diff = Z&wei Dateien vergleichen
diff-needs-two = Den Cursor in jedem Panel auf eine Datei setzen
diff-not-here = { $name } ist keine Datei auf dieser Platte
diff-too-big = { $name } ist größer als { $most }
diff-same = Die beiden Dateien sind gleich
diff-where = Block { $at } von { $count } · -{ $removed } +{ $added } Zeilen
diff-no-next = Der letzte Unterschied
diff-no-previous = Der erste Unterschied
diff-hint = Alt+↑↓ Blöcke · Tab Seite · F5 übernehmen · F2 schreiben · W Leerzeichen · C Groß/Klein · E Zeilenenden · Esc schließen
diff-hint-git = Alt+↑↓ Blöcke · S Block vormerken · Tab Seite · F5 übernehmen · F2 schreiben · Esc schließen
diff-hint-git-staged = Alt+↑↓ Blöcke · S Block aus dem Index · Tab Seite · F5 übernehmen · F2 schreiben · Esc schließen
diff-stage-edited = der Diff hat ungeschriebene Übernahmen: zuerst F2, dann S
diff-merged-left = Die linke Datei hat diesen Unterschied übernommen
diff-merged-right = Die rechte Datei hat diesen Unterschied übernommen
diff-no-hunk-here = Kein Unterschied oben am Bildschirm — Alt+↓ bringt einen hierher
diff-nothing-to-write = Es wurde nichts übernommen
diff-written = { $count } geschrieben
diff-unwritten = Eine Übernahme ist nicht geschrieben — F2 schreibt sie, noch einmal Esc verwirft sie
diff-ignore-space-on = Leerzeichen ignoriert
diff-ignore-space-off = Leerzeichen zählen wieder
diff-ignore-case-on = Groß- und Kleinschreibung ignoriert
diff-ignore-case-off = Groß- und Kleinschreibung zählt wieder
diff-ignore-eol-on = Zeilenenden ignoriert
diff-ignore-eol-off = Zeilenenden zählen wieder

grab-hint = ↑↓←→ bewegen · Umschalt+↑↓←→ markieren · Ctrl+A alles · Enter kopieren · Esc schließen
grab-copied = { $rows } Bildschirmzeilen kopiert

# ------------------------------------------------- die Codepages (SPEC 05 §1.12)
cp-title = Codepages
cp-name-title = Wie diese Codepage heißen soll
cp-set = Gelesen als { $name }
menu-panel-names = Codepage der Namen…
names-title = Namen lesen als
names-system = Wie das System sie angibt
names-set = Namen gelesen als { $name }
names-system-set = Namen wie das System sie angibt
names-not-here = Ein Name auf diesem System sind keine Bytes, es gibt also keine zweite Lesart
cp-marked = { $name } gehört zu denen, die F8 durchgeht
cp-unmarked = { $name } gehört nicht mehr zu denen, die F8 durchgeht
cp-hint = ↑↓ wählen · Enter so lesen · Einfg für F8 markieren · F4 umbenennen · Esc schließen

# ------------------------------------------------- die benannten Maskengruppen (SPEC 02 §5b)
mg-title = Maskengruppe
mg-name = Name
mg-mask = Masken
mg-hint = Einfg fügt eine Gruppe hinzu, Entf entfernt sie, F4 bearbeitet · ein `·` markiert die drei eingebauten
mg-saved = <{ $name }> gilt; Einstellungen speichern schreibt es in die Datei
mg-removed = <{ $name }> ist weg
mg-bad = <{ $name }>: { $problem }
mg-needs-name = ein Name ohne Leerzeichen, Kommas und spitze Klammern
mg-needs-mask = eine Gruppe, die für nichts steht, trifft nichts
mg-built-in = <{ $name }> ist eine der drei eingebauten · F4 gibt ihr andere Masken

sg-title = Sortiergruppe
sg-name = Name
sg-mask = Masken
sg-place = Wohin
sg-place-front = vor den übrigen
sg-place-back = hinter den übrigen
sg-hint = Einfg fügt ein Band hinzu, Entf entfernt eines, F4 bearbeitet, Strg+↑↓ verschiebt
sg-saved = Das Band gilt; Einstellungen speichern schreibt es
sg-removed = { $name } ist weg
sg-bad = { $name }: { $problem }
sg-needs-mask = ein Band ohne Maske würde nichts enthalten
hl-name = Name
hl-mask = Maske oder Masken
hl-kind = Gilt für
hl-kind-both = Dateien und Ordner
hl-kind-file = nur Dateien
hl-kind-dir = nur Ordner
hl-size = Größe (1M-10M, -4K, 1G-)
hl-age = Alter in Tagen (<=1, >=30, 7-30)
hl-date-field = Das Alter betrifft
hl-attrs = Attribute (h r x s b, - für "darf nicht")
hl-fg = Farbe (#d7875f, red, file.archive)
hl-bg = Hintergrund, genauso geschrieben
hl-hint = ↑↓ wählen · Enter/F4 ändern · Ins neu · Del löschen · Ctrl+↑↓ verschieben · Esc zu
hl-saved = Gruppe gilt · Optionen → Einstellungen speichern schreibt sie in die Datei
hl-removed = { $name } entfernt · Optionen → Einstellungen speichern schreibt es in die Datei
hl-bad = { $name }: { $problem }
filter-off = Kein Filter: alle Namen
filter-hint = ↑↓ wählen · Enter verwenden · F4 bearbeiten · Einfg neu · Entf entfernen · Esc schließen
filter-new = Neuer Filter
filter-edit = Filter
filter-name-hint = mit Namen bleibt er im Filtermenü; ohne wird er einmal verwendet
filter-applied = Filter aktiv: { $name }
filter-cleared = Filter aus
filter-saved = { $name } ist im Panel und im Filtermenü · Optionen → Einstellungen speichern schreibt ihn in die Datei
filter-removed = { $name } ist weg · Optionen → Einstellungen speichern schreibt es in die Datei
editor-no-block = es ist nichts ausgewählt
editor-unique = { $count } wiederholte Zeilen entfernt
editor-no-bracket = hier ist keine Klammer, oder keine passt zu ihr
editor-tabs-expanded = Tabulatoren in { $lines } Zeilen umgewandelt — das Speichern schreibt es
site-protocol = Protokoll
site-protocol-sftp = SFTP (über SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP mit TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV über HTTPS
site-auth = Anmeldung
site-auth-agent = SSH-Agent
site-auth-key = Schlüsseldatei
site-auth-ask = nach Kennwort fragen
site-key-file = Schlüsseldatei
site-start-dir = Entfernter Ordner
site-group = Gruppe
site-keepalive = Keep-alive (s)
site-compress = Komprimierung
site-page-session = Sitzung
site-page-environment = Umgebung
site-page-directories = Verzeichnisse
site-page-sftp = SFTP
site-password-note = Es wird kein Passwort gespeichert: es wird beim Verbinden erfragt.
site-env-eol = Zeilenenden umwandeln (noch nicht)
site-env-utf8 = Namen als UTF-8 erzwingen (noch nicht)
site-jump = Sprung-Host
site-jump-note = Erreicht über diesen Host: host, benutzer@host oder benutzer@host:port. Leer heißt direkt zum Server.
site-sftp-ciphers = Verschlüsselungsverfahren wählen (noch nicht)
btn-connect = Verbinden
site-needs-host = eine Verbindung braucht einen Host
site-needs-key = Schlüsseldatei wählen oder eine andere Anmeldung
site-bad-port = `{ $port }` ist keine Zahl
site-stored = Verbindung gespeichert: { $name } (Shift+F9 behält sie)
# ------------------------------------------------- Strukturfenster (SPEC 05 §1.8)
key-struct = Strukt
key-markdown = MD
key-table = Tabelle
key-filter = Filter
key-decompile = Dekomp
key-next = Weiter
key-prev = Zurück
key-codepages = CodeSt
key-goto = GeheZu
key-to-panel = Panel
key-blame = Blame
viewer-structure = Struktur
viewer-structure-hint = ↑↓ Zeile · Eingabe springen · Tab Inhalt · F11 schließen
viewer-structure-hint-tab = Tab bringt die Tasten hierher
viewer-no-structure = nichts zu entschlüsseln: dieses Format kennt das Fenster nicht
viewer-no-target = diese Zeile zeigt auf keine Stelle der Datei
viewer-form-opened = Das Formular, als der Text, den Delphi dafür zeigt. F4 führt zu den Bytes zurück.
viewer-metadata-opened = Der ganze Text, zum Lesen gesetzt. F4 führt zu den Bytes zurück.
viewer-went-to = gesprungen zu { $offset }
viewer-in-panel = das Panel steht auf { $name }
viewer-not-in-panel = { $name } ist nicht in diesem Panel
viewer-no-more-files = keine weiteren Dateien im Panel
editor-unsaved-for-viewer = zuerst speichern: der Betrachter liest die Datei von der Platte
# ------------------------------------------------- Befehlszeile (SPEC 06 §7)
cmd-running = läuft: { $command }
prompt-panelize = Befehl, dessen Ausgabe die Dateien nennt
prompt-viewer-filter = Befehl, dessen Ausgabe statt der Datei gelesen wird
learn-title = Welche Tasten dieses Terminal sendet
learn-hint = Tasten drücken. Eine helle Zelle ist angekommen; zweimal Esc beendet.
learn-last = zuletzt: { $key } — { $command }
learn-unbound = an nichts gebunden
recode-title = Neu schreiben als
recode-what = { $count } Dateien
recode-from = Was sie jetzt sind
recode-to = Schreiben als
recode-auto = pro Datei erkennen
recode-eol = Zeilenenden
recode-eol-keep = unverändert lassen
recode-working = { $count } Dateien werden neu geschrieben
recode-done = { $changed } geschrieben, { $same } schon so, { $failed } abgelehnt
menu-recode = Neu schreiben als…
panelize-title = { $command }
panelize-done = { $count } Dateien, { $dropped } Zeilen übersprungen
panelize-nothing = { $command }: keine Zeile nennt eine vorhandene Datei
cmd-interrupted = Unterbrechung an den Befehl gesendet
cmd-done = { $command } beendet
cmd-failed = { $command } endete mit { $code }
cmd-busy = es läuft schon ein Befehl
prompt-apply = Befehl für jede der { $count } Dateien:
cmd-apply-no-ask = ein Befehl, der nach einem Wert fragt, lässt sich nicht Datei für Datei anwenden
cmd-apply-progress = { $command }  ({ $done } von { $total })
cmd-apply-stopped = nach { $done } von { $total } abgebrochen: { $command } ist fehlgeschlagen
cmd-not-remote = ein Befehl braucht ein lokales Verzeichnis; dieses Fenster ist auf einem Server
cmd-no-history = der Befehlsverlauf ist leer
cmd-no-previous = es gibt kein Zurück
cmd-no-such-dir = { $path }: kein solches Verzeichnis
cmd-no-such-path = { $path }: keine solche Datei oder Verzeichnis
cmd-no-selection = nichts ausgewählt
cmd-condition-false = nicht ausgeführt: { $command }
cmd-screen-cleared = Bildschirm geleert
cmd-stack-empty = der Verzeichnisstapel ist leer
cmd-stack-cleared = Verzeichnisstapel geleert
cmd-pushed = { $count } Verzeichnisse auf dem Stapel
cmd-environment = Umgebung
cmd-set = { $name } = { $value }
cmd-not-set = { $name } ist nicht gesetzt
user-screen-title = Ausgabe — { $lines } Zeilen, ab Zeile { $at }
user-screen-empty = (noch nichts ausgeführt)
viewer-no-branch = diese Zeile hat keine Adresse zum Folgen
viewer-no-back = es gibt kein Zurück

# ------------------------------------------------- Dateisuche (SPEC 06 §1)
folders-title = Ordner finden
folders-reading = Die Ordner werden gelesen…
folders-read = { $count } Ordner
folders-capped = { $count } Ordner, so weit das Limit reicht
folders-local-only = die Ordnerliste ist die dieses Rechners; ein Server hat seinen Verlauf und die Favoriten
find-title = Datei suchen
find-mask = Dateimaske oder Masken
find-size = Größe (100M, 1M-1G, -4K)
find-text = Enthält den Text
find-case = Groß-/Kleinschreibung
find-words = Nur ganze Wörter
find-hex = Der Text oben ist ein Hex-Muster
find-not = Dateien, die ihn NICHT enthalten
find-scope = Suchen in
find-scope-panel = diesem Ordner
find-scope-tree = diesem Ordner und darunter
find-scope-selection = den markierten Ordnern
find-scope-root = ab der Wurzel dieses Laufwerks
find-scope-other = dem anderen Panel und darunter
find-links = Symbolischen Verknüpfungen folgen
find-dirs = Ordner auch in den Ergebnissen
find-archives = In Archiven
find-results = { $count } gefunden · { $files } Dateien und { $dirs } Ordner geprüft
find-done = { $count } in { $files } Dateien gefunden · { $elapsed }
find-none = nichts gefunden
find-stopped = nach { $files } Dateien abgebrochen
find-hint = ↑↓ wählen · Enter hingehen · F3 ansehen · F4 bearbeiten · F5 ins Panel · Esc stoppen
find-hint-done = ↑↓ wählen · Enter hingehen · F3 ansehen · F4 bearbeiten · F5 ins Panel · Esc schließen
find-panelised = { $count } Ergebnisse im Panel
find-needs-mask = eine Maske wird gebraucht: `*` heißt jede Datei
find-bad-size = `{ $text }` ist keine Größe — { $error }

# ------------------------------------------------- replace across files
replace-title = In Dateien ersetzen
replace-needle = Suchen nach
replace-with = Ersetzen durch
replace-case = Groß-/Kleinschreibung
replace-words = Nur ganze Wörter
replace-pattern = Der Text oben ist ein regulärer Ausdruck
replace-hex = Beide sind Hex-Bytes
replace-scope = { $count } Dateien markiert
replace-needs-needle = es gibt nichts zu suchen
replace-confirm-title = Ersetzen
replace-confirm-body = { $count } Vorkommen in { $files } Dateien würden sich ändern.
replace-nothing = nichts zu ersetzen: keine Datei würde sich ändern
replace-done = { $count } Vorkommen in { $files } Dateien ersetzt

# ── Archive (SPEC 04 §4) ────────────────────────────────────────────────────────────────
arc-writing-back = { $name } geht zurück in das Archiv
arc-move-out = um etwas aus einem Archiv zu holen, kopiere es mit F5
arc-add-files-only = einzeln kommen nur Dateien hinein; Shift+F1 legt ein ganzes Verzeichnis hinein

rename-title = Mehrere umbenennen
rename-name = Name
rename-ext = Erweiterung
rename-search = Suchen nach
rename-replace = Ersetzen durch
rename-regex = Regulärer Ausdruck ($1 im Ersatz)
rename-case = Buchstaben
rename-case-same = wie sie sind
rename-case-lower = klein
rename-case-upper = GROSS
rename-case-title = Jedes Wort Groß
rename-case-sentence = Nur der erste Buchstabe
rename-counter = Zähler: Start, Schritt, Stellen
rename-fields = [N] Name  [E] Erweiterung  [N1-3] Teil  [C] Zähler  [d] [t] Datum, Zeit  [P] Ordner
rename-not-in-archive = mehrere Dateien werden in einem Ordner umbenannt, nicht in einem Archiv: dort benennt F6 einen Eintrag um
rename-bad-counter = der Zähler sind drei ganze Zahlen, Start, Schritt, Stellen: { $counter } ist es nicht
rename-cannot = kann nicht umbenennen: { $why }
rename-problem-empty = kein Name übrig
rename-problem-invalid = kein Name, den eine Datei haben kann
rename-problem-twice = eine andere Datei bekommt diesen Namen auch
rename-problem-exists = eine Datei dieses Namens ist schon da
rename-more = … und { $count } weitere
rename-blocked =
    { $count ->
        [one] eine Datei kann ihren neuen Namen nicht bekommen: nichts wird umbenannt, bis sie es kann
       *[other] { $count } Dateien können ihre neuen Namen nicht bekommen: nichts wird umbenannt, bis sie es können
    }
rename-summary = { $count } von { $total } ändern sich
rename-nothing = kein Name ändert sich
rename-nothing-to-undo = es gibt keine Mehrfach-Umbenennung zum Rückgängigmachen
rename-done =
    { $count ->
        [one] eine Datei umbenannt · rückgängig im Menü Dateien
       *[other] { $count } Dateien umbenannt · rückgängig im Menü Dateien
    }
rename-undone =
    { $count ->
        [one] ein Name zurückgesetzt
       *[other] { $count } Namen zurückgesetzt
    }
pack-title = Zum Archiv hinzufügen
pack-name = Archiv
pack-format = Format
pack-level = Kompression
pack-solid = Solide Blöcke (7z: kleiner, langsamer für eine Datei)
pack-delete-after = Dateien nach dem Archivieren löschen
pack-level-normal = normal
pack-level-fast = schnell
pack-level-best = maximal
pack-scope = { $count } Dateien ausgewählt
pack-needs-name = das Archiv braucht einen Namen
pack-format-read-only = { $format } wird hier gelesen, nicht geschrieben: wähle ein anderes Format oder einen anderen Namen
pack-single-one-file = ein { $format } enthält genau eine Datei: wähle eine aus, oder nimm { $tar } für einen Ordner oder mehrere Dateien
unpack-title = Aus Archiv entpacken
unpack-to = Entpacken nach
unpack-scope = aus { $name }
unpack-not-an-archive = kein Archiv, das wir lesen können

# ── die besonderen Panel-Arten (SPEC 02 §7) ─────────────────────────────────────────────
panel-quick-view = Schnellansicht
panel-info = Informationen
panel-tree = Baum
compare-done = { $left } links, { $right } rechts unterscheiden sich
cmp-subdirs = In die Unterverzeichnisse gehen
cmp-depth = Wie tief (0 = bis ganz unten)
cmp-trees-title = Was sich unterscheidet
cmp-trees-working = vergleiche die Bäume: { $seen } Dateien, { $read } gelesen
cmp-trees-done = { $differ } unterschiedlich, { $left } nur links, { $right } nur rechts, von { $pairs } Paaren
cmp-trees-same = die beiden Bäume sind gleich ({ $pairs } Paare, { $same } geprüft)
menu-duplicates = Duplikate finden
dup-title = Doppelte Dateien
dup-working = suche Duplikate: { $seen } Dateien, { $hashed } gelesen
dup-done = { $groups } Gruppen von Kopien, { $files } Dateien, { $wasted } verschwendet
dup-none = hier ist nichts eine Kopie von etwas anderem ({ $seen } Dateien)
dup-stopped = nach { $seen } Dateien angehalten
dup-partial = vorzeitig angehalten
cmp-title = Vergleich
cmp-case = Namen mit Groß- und Kleinschreibung vergleichen
cmp-size = Eine andere Größe ist ein Unterschied
cmp-time = Eine andere Zeit ist ein Unterschied
cmp-seconds = Zeiten mit so vielen Sekunden Abstand sind derselbe Moment
cmp-contents = Die Bytes lesen
cmp-when-time = Nur lesen, wenn die Zeiten abweichen
cmp-fragment = Nur die ersten so viele KiB lesen (0 = alles)
cmp-ignore-eol = Zeilenenden allein sind kein Unterschied
cmp-selected = Nur das Ausgewählte
cmp-stop = Beim ersten Unterschied anhalten
cmp-newer = Nur die neuere Kopie auswählen
menu-compare-setup = Vergleich mit Optionen
compare-contents-eol-ignored = { $count } unterscheiden sich nur in den Zeilenenden, ignoriert
compare-contents-trusted = { $count } ohne Lesen als gleich angenommen
compare-contents-working = Inhalte werden verglichen: { $done } von { $total }
compare-contents-done = { $differ } von { $pairs } Paaren unterscheiden sich
compare-contents-same = alle { $pairs } Paare sind identisch
compare-contents-none = keine Datei hat auf beiden Seiten denselben Namen
compare-contents-local = der Inhaltsvergleich braucht beide Fenster auf einem lokalen Dateisystem
compare-contents-eol = { $count } nur in den Zeilenenden
compare-contents-only = { $count } nur auf einer Seite
compare-contents-unread = { $count } nicht lesbar
compare-contents-stopped = Vergleich abgebrochen
quick-view-directory = { $name } ist ein Verzeichnis — F3 misst es
quick-view-not-here = die Bytes sind nicht hier: F3 holt sie zuerst
info-path = Pfad
info-kind = Art
info-kind-local = ein Verzeichnis auf diesem Rechner
info-kind-remote = auf { $host }
info-kind-archive = in { $name }
info-kind-list = eine Dateiliste
info-device = Laufwerk
info-folder = Ordner
info-device-value = { $total }, { $free } frei, Cluster { $cluster }
info-slack = Verschnitt
info-filesystem = Dateisystem
info-mount = Eingehängt
info-git = Repository
info-git-clean = wie im Index
info-git-differing = { $count } von { $tracked } weichen vom Index ab
info-git-uncounted = nicht gezählt
info-git-merge = Merge läuft
info-git-rebase = Rebase läuft
info-git-cherry-pick = Cherry-Pick läuft
info-git-revert = Revert läuft
info-git-bisect = Bisect läuft
info-memory = Speicher
info-memory-value = { $free } frei von { $total }
info-memory-total = { $total }
info-contents = Enthält
info-contents-value = { $files } Dateien, { $dirs } Verzeichnisse, { $bytes }
info-selected = Ausgewählt
info-selected-value = { $count } Dateien, { $bytes }
info-cursor = Cursor
info-size = Größe
info-modified = Geändert
info-description = Beschreibung
key-mark = Marke
key-asm = Asm
key-block = Block
key-screens = Schirme
key-site-edit = SitzBrb
key-site-new = SitzNeu
key-site-forget = SitzLös
key-ai-edit = Ändern
key-ai-new = Neu
key-ai-remove = Entf
key-ai-forget = Entf
key-ai-threads = Verlauf
hex-needs-a-file = dieser Puffer hat keine Datei, aus der Bytes kämen
hex-save-first = zuerst speichern: die beiden Editoren können einander keine ungespeicherten Änderungen übergeben

hex-block-title = Block
hex-op-write = { $size } in eine Datei schreiben
hex-op-read = Eine Datei bei { $at } einlesen
hex-op-insert = Nullbytes bei { $at } einfügen
hex-op-copy = Bytes von anderswo nach { $at } kopieren
hex-op-move = Bytes von anderswo nach { $at } verschieben
hex-op-fill = { $size } mit einem Byte füllen
hex-op-xor = { $size } mit einem Schlüssel exklusiv-odern
hex-op-delete = { $size } löschen — aus { $was } wird { $becomes }
hex-write-title = Block schreiben nach
hex-read-title = Welche Datei einlesen
hex-fill-title = Mit welchem Byte füllen (hex)
hex-xor-title = Mit welchem Schlüssel (hex)
hex-insert-title = Wie viele Nullbytes
hex-copy-title = Woher kopieren — Position und Länge (hex)
hex-move-title = Woher verschieben — Position und Länge (hex)
hex-not-hex = das sind keine Hexziffern
hex-not-a-count = das ist keine Byteanzahl
hex-not-a-range = das ist keine Position mit einer Länge in Hexadezimal
hex-range-outside = dieser Bereich liegt außerhalb der Datei ({ $size })
hex-move-into-itself = ein Block kann nicht in sich selbst verschoben werden
hex-filled = { $size } gefüllt
hex-xored = { $size } exklusiv-odert — das ist keine Verschlüsselung
hex-written = { $size } nach { $name } geschrieben
hex-read-done = { $size } aus { $name } gelesen
hex-inserted = { $size } Nullen eingefügt
hex-deleted = { $size } gelöscht
hex-device = { $name }: ein Gerät mit { $size }, nur lesend geöffnet
asm-title = Bei { $at } assemblieren ({ $arch })
asm-no-arch = { $arch } ist keine Architektur, die wir assemblieren können
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } Bytes länger)
screens-title = Bildschirme
screens-panels = Fenster
quit-title = FinalCommander verlassen
quit-body = Beenden?
screens-quit-title = Ungespeicherte Änderungen
screens-quit-body = Diese Bildschirme enthalten ungeschriebene Änderungen:
macro-recording = Makro wird aufgezeichnet · Ctrl+. beendet
macro-empty = es wurde nichts aufgezeichnet
macro-dropped = Aufzeichnung verworfen
macro-key-title = Welche Taste
macro-key-body = { $steps } Schritte aufgezeichnet. Drücken Sie die Taste für dieses Makro, oder Esc zum Verwerfen.
macro-describe = Wofür ist das Makro auf { $key }
macro-saved = Makro auf { $key } gespeichert, { $steps } Schritte
macro-none = noch keine Makros · Ctrl+. zeichnet eins auf
macro-list-title = Makros
macro-play-title = Wie soll es ablaufen
macro-play-once = Einmal, ab der Cursorzeile
macro-play-each = Einmal für jede ausgewählte Datei
macro-sure-title = Dieses Makro ausführen
macro-sure-body = Das Makro auf { $key } kopiert, verschiebt, löscht oder überschreibt. Ausführen?
macro-too-deep = ein Makro kann sich nicht endlos selbst aufrufen
plugin-list-title = Plugin-Befehle
plugin-none-title = Keine Plugins installiert
plugin-none-open = Plugin-Ordner öffnen ({ $dir })
plugin-none-help = Wie Plugins funktionieren und wo es welche gibt
plugin-help-heading = Plugins
plugin-why-off = ausgeschaltet unter Optionen → Plugin-Konfiguration
plugin-why-panel = nur in den Panels
plugin-why-viewer = nur im Betrachter
plugin-why-editor = nur im Editor
plugin-why-local = braucht einen Ordner auf den Laufwerken dieses Rechners
plugin-why-files = braucht eine Datei unter dem Cursor oder markiert
plugin-why-masks = nur für { $masks }
plugin-why-bad-mask = seine Maske ist falsch: { $error }
plugin-why-here = braucht { $names } in diesem Ordner
plugin-why-inside = braucht { $names } hier oder in einem Ordner darüber
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = für { $count } ausgewählte Datei(en)
plugin-failed = { $id }: { $why }
plugin-unknown-command = ein Plugin verlangte `{ $id }`, das ist kein Befehl
hex-copied = { $size } von { $from } kopiert
hex-moved = { $size } von { $from } verschoben
menu-calculator = Taschen&rechner
menu-terminal = Terminal
calc-title = Rechner
calc-decimal = dezimal
calc-hex = hex
calc-octal = oktal
calc-binary = binär
calc-characters = Zeichen

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Archivbefehle
arc-password = Passwort für { $name }
arc-op-test = { $name } testen
arc-op-extract-here = { $name } hierher entpacken
arc-op-extract-to = { $name } entpacken nach…
arc-op-delete = { $name } löschen
arc-op-add = Dateien zu einem Archiv hinzufügen…
job-test = Archiv wird getestet
msg-dirs-first-on = Ordner zuerst
msg-dirs-first-off = Ordner zwischen den Dateien
msg-selected-first-on = markierte Dateien zuerst
msg-selected-first-off = markierte Dateien in der Reihenfolge

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Platte
drive-removable = Wechsel
drive-optical = Optisch
drive-network = Netzwerk
drive-memory = Speicher
drive-home = Persönlich
drive-connection = Verbindung
drive-objects = Objektspeicher
drive-netbox = NetBox: gespeicherte Sitzungen
drive-processes = Prozessliste
drive-temp = Temporäres Panel
temp-added = { $count } ins temporäre Panel gelegt
proc-none = die Prozessliste ist hier nicht lesbar
drive-programs = Installierte Programme
drive-services = Dienste
svc-none = dieser Rechner hat keine Dienstverwaltung, die wir fragen können
temp-not-a-file = die Zeilen hier sind keine Dateien: es gibt nichts zu löschen
svc-card = Dienst
svc-card-id = Name
svc-card-name = Angezeigt als
svc-card-state = Zustand
svc-card-description = Beschreibung
svc-card-pid = Prozess
svc-state-running = läuft
svc-state-stopped = gestoppt
svc-state-failed = fehlgeschlagen
svc-state-other = sonstiges
key-service-start = Start
key-service-restart = Neustart
key-service-stop = Stopp
svc-ask-stop = { $name } stoppen?
svc-ask-restart = { $name } neu starten?
btn-service-stop = Stoppen
btn-service-restart = Neu starten
svc-working = { $name }: frage das System…
svc-started = { $name } gestartet
svc-stopped = { $name } gestoppt
svc-restarted = { $name } neu gestartet
svc-needs-root = { $name }: dafür braucht es root — das Programm fragt nicht an deiner Stelle danach
svc-failed = { $name }: { $error }
prog-none = hier sagt nichts, was installiert ist
prog-card = Installiertes Programm
prog-no-uninstall = nichts sagt, wie { $name } entfernt wird
prog-uninstall-ready = der Befehl, der { $name } entfernt, steht in der Befehlszeile, nicht ausgeführt
prog-card-name = Name
prog-card-version = Version
prog-card-publisher = Hersteller
prog-card-size = Belegt
prog-card-installed = Installiert
prog-card-location = Ordner
prog-card-uninstall = Entfernt mit
drive-sockets = Socket-Liste
sock-none = hier lauscht nichts, oder die Tabelle ist nicht lesbar
sock-no-process = niemand sagt, welcher Prozess diesen Socket hält
proc-card = Prozess
proc-gone = { $name } läuft nicht mehr
proc-card-name = Name
proc-card-pid = PID
proc-card-parent = Elternprozess
proc-card-user = Benutzer
proc-card-state = Zustand
proc-card-threads = Threads
proc-card-memory = Speicher
proc-card-mapped = Zugeordnet
proc-card-bits = Bit
proc-card-started = Gestartet
proc-card-exe = Programm
proc-card-cwd = Verzeichnis
proc-card-command = Befehlszeile
proc-card-group = Gruppe
proc-card-sid = SID
proc-card-priority = Priorität
proc-card-session = Sitzung
proc-card-uptime = Läuft seit
proc-card-cpu-user = Prozessorzeit, eigene
proc-card-cpu-kernel = Prozessorzeit, im Kernel
proc-card-handles = Offene Handles
proc-card-faults = Seitenfehler
proc-card-private = Privater Speicher
proc-card-swap = Ausgelagert
proc-card-io-read = Gelesen
proc-card-io-write = Geschrieben
proc-card-memory-peak = Arbeitssatz-Spitze
proc-card-private-resident = Arbeitssatz, privat
proc-card-swap-peak = Auslagerungsdatei-Spitze
proc-card-virtual = Virtueller Speicher
proc-card-virtual-peak = Virtueller Speicher, Spitze
proc-card-paged-pool = Ausgelagerter Pool
proc-card-nonpaged-pool = Nicht ausgelagerter Pool
proc-card-io-other = Andere E/A
proc-card-io-read-ops = Leseoperationen
proc-card-io-write-ops = Schreiboperationen
proc-card-io-other-ops = Andere Operationen
proc-kill-title = Prozess beenden
proc-kill-ask = { $name } [{ $pid }] beenden? Was nicht gespeichert ist, geht verloren.
proc-killing = Beende { $name }…
proc-killed = { $name } beendet
proc-kill-failed = { $name } lässt sich nicht beenden: { $why }
proc-card-env = Umgebung
proc-card-modules = Geladene Module
proc-card-version = Version
proc-card-description = Beschreibung
drives-hint = ↑↓ wählen · 0-9 Panels · Ctrl+1..9 Spalten · F9 alle · Ctrl+R neu lesen
drives-title = Laufwerkwahl: was gezeigt wird
drives-col-type = den Typ
drives-col-device = das Gerät
drives-col-label = die Bezeichnung
drives-col-fs = das Dateisystem
drives-col-size = die Größen
drives-col-removable = die Wechseldatenträger
drives-col-panels = die Panels, die keine Laufwerke sind
drives-col-optical = die optischen Laufwerke
drives-col-network = die Netzlaufwerke
drives-shown = { $what } wird gezeigt
drives-hidden = { $what } wird ausgeblendet
drives-reread = Laufwerke neu gelesen
bookmarks-hint = Eingabe gehe · Einfg merken · Entf entfernen · F4 benennen · Esc schließen

# The search dialog (F7), which asks the whole question in one place.
search-title = Suchen
search-for = Suchen nach
search-mode = Als
search-mode-text = Text
search-mode-hex = Hex
search-mode-regex = regulärer Ausdruck
search-case = Groß-/Kleinschreibung
search-words = Nur ganze Wörter
btn-find-next = Weitersuchen
btn-find-prev = Rückwärts suchen
help-chapter-about = Was das ist
help-chapter-panels = Die Panels
help-chapter-files = Dateien: kopieren, verschieben, suchen, Archive
help-chapter-viewer = Der Betrachter
help-chapter-editor = Der Editor, die Hex-Ansicht und der Assembler
help-chapter-network = Server
help-chapter-config = Konfiguration, Tasten und Farben
help-chapter-keys = Die Tasten, wie sie jetzt belegt sind
help-chapter-commands = Alle Befehle und ihre Tasten
help-chapter-options = Alle Einstellungen und ihre Werte
help-here-panels = Die Panels
help-here-viewer = Der Betrachter
help-here-editor = Der Editor
help-here-hex = Der Hex-Editor
help-here-dialog = Dieser Dialog
help-in-english = Dieses Kapitel ist noch nicht in Ihre Sprache übersetzt und folgt daher auf Englisch. Die Tasten und Einstellungen, die es nennt, sind in jeder Sprache dieselben.
help-about-byline = FinalCommander { $version } — von Matteo Baccan
key-ok = OK
key-cancel = Abbruch
key-dialog-move = Verschb
key-tree = Baum
key-config = Konfig
browse-local-only = eine Datei wird auf diesem Rechner ausgewählt; ein Pfad auf einem Server wird eingegeben, und Tab vervollständigt ihn
key-browse = Durchs
table-prompt-filter = Zeilen mit
table-sorted = nach { $column } sortiert
table-sorted-back = nach { $column } sortiert, umgekehrt
table-unsorted = wieder in der Reihenfolge der Datei
table-filtered = { $rows } Zeilen übrig
table-unfiltered = wieder alle Zeilen
table-cut-sorted = von den ersten { $rows } Zeilen, mehr wurde nicht gelesen
table-too-many = zu viele Datensätze, um sie hier zu sortieren oder zu filtern
table-copied =
    { $rows ->
        [one] eine Zeile in der Zwischenablage
       *[other] { $rows } Zeilen in der Zwischenablage
    }
table-headed = die erste Zeile ist die Überschrift
table-not-headed = die erste Zeile sind Daten: die Spalten heißen nach ihrer Nummer
table-named-fields = dieses Format benennt seine Felder selbst: es gibt keine Überschrift zu raten

# Splitting and combining (SPEC 03 §9)
split-title = Datei teilen
split-target = Nach
split-size = Jedes Stück
split-size-hint = 1440K, 700M, 4G — das letzte Stück ist der Rest
split-count = Oder in
split-count-hint = Teile — eine Zahl hier geht vor der Größe
split-bad-count = { $count } ist keine Anzahl von Teilen: zwei oder mehr
split-local-file = Teilen und Zusammenfügen gehen mit einer Datei auf diesem Rechner
split-bad-size = „{ $size }“ ist keine Größe: schreiben Sie 700M oder 4G
combine-not-a-piece = Das ist kein Stück: die Stücke einer geteilten Datei enden auf .001, .002, … oder haben eine .crc
pieces-working = Teilen oder Zusammenfügen: { $done } von { $total }
pieces-cancelled = Angehalten, und das Geschriebene ist wieder entfernt
split-done = { $parts } Stücke geschrieben, daneben { $list }
combine-done-checked = { $name } aus { $parts } Stücken zusammengefügt, { $bytes }, passt zu seiner .crc
combine-done-unchecked = { $name } aus { $parts } Stücken zusammengefügt, { $bytes } — keine .crc zum Prüfen
pieces-failed = Nicht erledigt: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Alle Dateien unter { $path } werden gelesen…
branch-title = Zweig: { $path }
branch-done = { $count } Dateien aus allen Ordnern darunter · Strg+Alt+B geht zurück
branch-truncated = { $count } Dateien, und dort hat das Lesen aufgehört: der Zweig ist größer, als eine Liste sinnvoll sein kann
branch-not-here = Die Zweigansicht ist die eines Ordners, und dieses Feld ist eine Liste

## The release check (SPEC 11 §5).
menu-check-update = Nach Updates su&chen
opt-update-check = Einmal am Tag nach einer neuen Version suchen
btn-open-page = Seite öffnen
update-checking = Frage GitHub nach der neuesten Version…
update-title = Updates
update-available = FinalCommander { $version } ist erschienen (dies ist { $current }).
update-available-status = FinalCommander { $version } ist erschienen: Optionen → Nach Updates suchen
update-current = Dies ist die neueste Version, { $version }.
update-failed = Die neueste Version konnte nicht abgefragt werden: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Doppelte Dateien suchen
dupes-title = Doppelte Dateien
dupes-recursive = In Unterordnern suchen
dupes-min-size = Kleinste Datei
dupes-min-size-hint = Eine Größe wie in den Filtern: 1, 100K, 5M. Leere Dateien werden nie verglichen.
dupes-select = Alle Kopien außer der ältesten markieren
btn-search = Suchen
dupes-not-here = Die Suche nach Duplikaten beginnt in einem Ordner oder bei der Auswahl.
dupes-bad-size = Keine Größe: { $size }
dupes-looking = Suche doppelte Dateien…
dupes-progress = Duplikate: Durchgang { $pass } von 3, { $done }/{ $total }
dupes-none = Keine Duplikate unter { $count } Dateien.
dupes-panel = Duplikate: { $groups } Gruppen
dupes-done = { $groups } Gruppen von Duplikaten, { $size } in den Kopien.
dupes-truncated = Vorzeitig beendet: bisher { $groups } Gruppen, { $size } in den Kopien.

## SQLite queries (SPEC 04 §4b).
sql-empty = Eine Abfrage nach sql: schreiben, etwa sql: SELECT * FROM t
sql-no-database = sql: braucht eine SQLite-Datenbank: hineingehen oder den Cursor auf eine setzen.
sql-running = Abfrage läuft…
sql-failed = Die Abfrage ist fehlgeschlagen: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Kein KI-Dienst eingerichtet: einen [[ai]]-Block in fcmd.toml anlegen (name, url, model, api, token_env).
ai-title = Eine KI fragen
ai-no-thread = keine Antwort zum Fortsetzen: zuerst mit `ai:` fragen oder den Cursor im Antwortordner auf eine setzen
ai-nothing-to-explain = hier gibt es nichts zu erklären: eine Datei im Betrachter oder Editor öffnen
ai-explain-question = Was ist das, aus { $name }? Kurz erklären.
ai-tokens = { $in } Tokens hinein, { $out } heraus
ai-none-panel = Noch kein KI-Dienst — F7 fügt einen hinzu
ai-service-removed = { $name } entfernt
ai-probing = Anmeldung bei { $name } auf { $host }…
ai-probe-ok = { $host } antwortete HTTP { $status } in { $ms } ms, { $models } Modelle — Frage eingeben und Enter drücken
ai-probe-no-model = (das Modell { $model } ist nicht dabei)
ai-row-context = Kontext
ai-row-conversation = Gespräch
ai-session-title = KI: { $name } · { $model } — { $files } Dateien im Kontext
ai-context-added = { $count ->
        [one] Eine Datei
       *[other] { $count } Dateien
    } im Kontext
ai-context-nothing = Nichts hinzuzufügen: nur Dateien auf dieser Platte kommen in den Kontext
ai-command-ready = der Befehl steht in der Zeile — vor Enter lesen
ai-commit-question = Schreibe eine Commit-Nachricht für die vorgemerkten Änderungen.
ai-staged-diff = der vorgemerkte Diff
ai-send-diff = Der vorgemerkte Diff geht an { $host }
ai-send-files = { $count } Dateien, { $size }, werden mit der Frage an { $host } gesendet.
ai-asking = Frage { $name }…
ai-busy = { $name } denkt nach… { $secs } s · Esc bricht ab
ai-conversation-new = Ein neues Gespräch: die nächste Frage beginnt es.
ai-live-empty = Eine Frage in die Befehlszeile schreiben: sie geht an { $name }, und das Gespräch erscheint hier.
ai-failed = Die KI hat nicht geantwortet: { $why }
ai-stopped = abgebrochen: die Frage an { $name } wurde verworfen, nichts wurde geschrieben
ai-context-title = Dateien im Kontext
ai-context-all = Alle
ai-context-dropped = { $count ->
        [one] eine Datei aus dem Kontext genommen
       *[other] { $count } Dateien aus dem Kontext genommen
    }
ai-context-hint = Enter oder Entf: aus dem Kontext nehmen — die Datei bleibt auf der Platte · Esc
ai-threads-title = Gespräche
ai-threads-hint = Enter: dieses Gespräch fortsetzen · Esc
ai-thread-resumed = { $name } wird fortgesetzt
ai-next-estimate = nächste ≈ { $tokens }
ai-next-over = nächste ≈ { $tokens }, mehr als das Modell fasst
ai-send-estimate = Etwa { $tokens } Token insgesamt, mit dem bisherigen Gespräch.
ai-send-estimate-of = Etwa { $tokens } Token insgesamt, mit dem bisherigen Gespräch, von { $max }, die das Modell fasst.
ai-send-over = Etwa { $tokens } Token insgesamt: mehr als die { $max }, die das Modell fasst, der Anfang kann abgeschnitten werden. F8 nimmt eine Datei heraus.
ai-send-question = Die Frage geht an { $host }.
btn-send = Senden

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Was geändert ist
git-log = Das Log
git-blame = Blame von { $name }
git-branches = Branches
git-local-only = git arbeitet mit einem Ordner auf dieser Platte.
git-running = Frage git…
git-failed = git: { $why }
git-clean = Nichts geändert.
git-status-title = Geändert: { $staged } bereitgestellt, { $unstaged } nicht
git-side-conflict = Konflikt
git-side-staged = vorgemerkt
git-side-changed = nicht vorgemerkt
git-side-untracked = unversioniert
git-kind-modified = geändert
git-kind-added = neu
git-kind-deleted = gelöscht
git-kind-renamed = umbenannt
git-kind-copied = kopiert
git-kind-type = Typ geändert
git-kind-conflict = zu lösen
git-as-tree = Änderungen als Baum zeigen
git-as-list = Änderungen als Liste zeigen
git-version-head = letzter Commit
git-version-index = vorgemerkt
git-version-disk = auf der Platte
git-discard-title = Änderungen verwerfen
git-discard-body = Die Änderungen { $count ->
        [one] einer Datei
       *[other] von { $count } Dateien
    } verwerfen?
git-discard-worktree = zurück auf den Index:
git-discard-both = zurück auf den letzten Commit:
git-discard-added = aus dem Index, bleibt auf der Platte:
git-discard-trash = in den Papierkorb:
git-discard-warning = Was auf den Index oder den letzten Commit zurückgeht, lässt sich nicht rückgängig machen.
git-hint = Enter: wählen · Esc: schließen

## The editor's character list and word completion.
charmap-title = Zeichen
charmap-hint = Tippen zum Eingrenzen (Zeichen oder Code, U+00E9) · Enter: einfügen · Esc: zurück
editor-no-completion = Kein anderes Wort der Datei beginnt so.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Kopiert: { $url }

## The system log (SPEC 06 §16).
menu-syslog = Systemprotokoll
syslog-reading = Lese das Systemprotokoll…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Zeilen entfernen, die passen (! davor: nur diese behalten)
editor-lines-deleted = Entfernte Zeilen: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = überschreiben, wenn die Größe abweicht

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Wenn eine nicht kopiert werden kann
copy-error-ask = Fragen
copy-error-skip = Überspringen und weiter
copy-error-stop = Kopieren anhalten

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Zeiten kopiert: Strg+Umschalt+V fügt sie im Dialog einer anderen Datei ein
attr-times-none = Noch nichts kopiert: zuerst Strg+Umschalt+C im Dialog einer Datei
attr-times-pasted = Zeiten eingefügt: OK setzt sie

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Ersetzungen
replace-tpl-hint = Enter führt aus · Einfg behält eine letzte unter einem Namen · Entf vergisst eine behaltene · Esc
replace-tpl-name = Diese Ersetzung behalten als
replace-tpl-kept = Behalten als {$name}
replace-tpl-empty = Noch keine Ersetzung: zuerst Strg+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Jeder Ordner behält seine Ansicht und Sortierung

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Geschwindigkeitsgrenze, KB/s (leer: keine)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (Mail, nur lesen)
site-protocol-imaps = IMAPS (Mail über TLS, nur lesen)

# The AI services as a list, like the sessions
ai-services-title = KI-Dienste
ai-services-hint = Enter: der, den ai: fragt · Einfg neu · F4 ändern · Entf löschen · Esc
ai-service-title = KI-Dienst
ai-field-name = Name
ai-field-api = Protokoll
ai-field-url = Adresse
ai-field-model = Modell
ai-field-token-env = Schlüssel in Variable
ai-field-token-note = Leer: der Schlüssel wird einmal pro Sitzung erfragt und nie gespeichert.
ai-field-system = Was ihm zuerst gesagt wird
ai-service-incomplete = Ein Dienst braucht Name und Adresse
ai-service-default = ai: fragt jetzt {$name}
ai-key-prompt = Der Schlüssel für {$name} (bis zum Beenden behalten)
ai-api-openai = OpenAI und kompatible
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (auf diesem Rechner)
menu-ai = KI-Dienste

# The secrets file
vault-password = Das Passwort der Geheimnisdatei von fcmd
vault-new = Diesen Schlüssel in fcmds verschlüsselter Geheimnisdatei behalten: Passwort wählen (leer: nur diese Sitzung)
vault-refused = Die Geheimnisdatei ließ sich nicht öffnen: {$why}

# Externe Programme
drive-tools = Externe Programme
drive-ai = KI
drive-git = Git
tools-found = gefunden
tools-missing = nicht installiert
tools-summary = { $found } von { $total } externen Programmen gefunden; F3 sagt, wofür jedes dient, F4 installiert ein fehlendes
tools-card = Externes Programm
tools-card-program = Programm
tools-card-where = Ort
tools-card-version = Version
tools-card-for = Verwendet für
tools-card-install = Installieren
tools-card-why = Warum fcmd es aufruft, statt es selbst zu tun: F1, „Programs from elsewhere“.
tools-install-any = das Paket installieren, das { $name } enthält, und prüfen, dass es im PATH liegt
tools-already = { $name } ist schon da
tools-for-archives = Archive: { $what }
tools-for-decompile = Shift+F6 im Betrachter auf { $what }
tools-for-git = das Git-Panel (Ctrl+Alt+G) und der Zweig in der Eingabeaufforderung
tools-for-video = Videobilder im Betrachter (geplant: noch ruft es nichts auf)
tools-for-open = einen Link im Browser öffnen
tools-for-syslog = Befehle → Systemprotokoll
tools-for-services = das Dienste-Panel
tools-card-site = Website
tools-no-package = Kein Paketmanager dieses Rechners hat bekanntermaßen { $name }. Die Download-Seite öffnen?
tools-install-ask = { $name } installieren, indem in einem Terminal ausgeführt wird:
tools-install-how = Im Terminal kann sudo nach Ihrem Passwort fragen. Ctrl+Alt+E führt zu den Panels zurück, Ctrl+R in diesem Panel sucht erneut.
tools-install-else = Falls es scheitert: { $url }
tools-install-failed = fcmd: { $name } wurde nicht installiert. Herunterladen von { $url }
tools-installing = { $name } wird in einem Terminal installiert: Ctrl+Alt+E führt zu den Panels zurück, Ctrl+R dort sucht erneut
btn-install = Installieren

# Plugin-Konfiguration
plugins-config-title = Plugins
plugins-config-hint = Enter: ein- oder ausschalten · F3 was es ist und mitbringt · Esc
plugins-broken = nicht geladen — F3 sagt warum
plugins-cannot-toggle = ein Plugin, das nicht geladen wurde, lässt sich nicht einschalten: F3 sagt, was fehlt
plugins-on = { $id } ist an: seine Befehle sind jetzt in F11
plugins-on-restart = { $id } ist an: seine Befehle sind jetzt in F11, der Rest ab dem nächsten Start
plugins-off = { $id } ist aus: seine Befehle sind aus F11 verschwunden
plugins-off-restart = { $id } ist aus: seine Befehle sind aus F11 verschwunden, der Rest beim nächsten Start
plugins-state-on = an
plugins-state-off = aus
plugins-card-id = Id
plugins-card-name = Name
plugins-card-version = Version
plugins-card-author = Autor
plugins-card-about = Beschreibung
plugins-card-dir = Ordner
plugins-card-state = Zustand
plugins-card-brings = Bringt
plugins-card-commands = Befehle (F11):
plugins-card-notes = Beim Laden gemeldet:
plugins-none = keine Plugins gefunden · sie liegen in { $dir }, je ein Ordner mit plugin.toml

# The git panel
git-file-log = Das Log von { $name }
git-commit = Bereitgestelltes committen…
git-pull = Pull
git-push = Push
git-pushing = Push läuft…
git-pulling = Pull läuft…
git-commit-prompt = Die Commit-Nachricht
git-commit-empty = Ein Commit braucht eine Nachricht.
git-already-on = { $branch } ist der aktuelle Branch.
git-switch-ask = Zu { $branch } wechseln? Die Dateien, in denen sich die Branches unterscheiden, ändern sich auf der Platte.
git-done = git: { $said }
git-branches-title = Branches: { $count }
git-log-title = Log: { $count }
git-log-of-title = Log von { $of }: { $count }
btn-switch = Wechseln
key-git = Git
key-git-diff = Diff
key-git-stage = Stage
key-git-unstage = Unstage
key-git-discard = Verwerfen
key-git-commit = Commit
key-git-show = Zeigen
key-git-log = Log
key-git-changes = Änder.
key-git-pull = Pull
key-git-push = Push
key-git-branches = Zweige

undo-title = Rückgängig
btn-undo = Rückgängig
menu-undo = Letzten Vorgang rückgängig
menu-strip-meta = Bild-Metadaten entfernen…
undo-nothing = nichts rückgängig zu machen: kein Kopieren, Verschieben, Löschen in den Papierkorb oder neuer Ordner seit dem Start
undo-copy =
    { $count ->
        [one] Die kopierte Datei in den Papierkorb legen:
       *[other] Die { $count } kopierten Dateien in den Papierkorb legen:
    }
undo-move =
    { $count ->
        [one] Die Datei zurückverschieben:
       *[other] Die { $count } Dateien zurückverschieben:
    }
undo-trash =
    { $count ->
        [one] Die Datei aus dem Papierkorb zurückholen:
       *[other] Die { $count } Dateien aus dem Papierkorb zurückholen:
    }
undo-mkdir = Den neuen Ordner entfernen, wenn er noch leer ist:
strip-title = { $count ->
        [one] Metadaten eines Bildes
       *[other] Metadaten von { $count } Bildern
    }
strip-all = Alles — Text, EXIF, XMP, IPTC, Kommentare, C2PA
strip-ai = Nur was ein Bildmodell geschrieben hat — Prompt, Workflow, Einstellungen
strip-camera = Alles außer dem EXIF der Kamera und dem Copyright — kein GPS
strip-hint = Enter: entfernen; das Bild wird nicht neu kodiert, Strg+Alt+Z stellt es wieder her · Esc
strip-nothing = kein PNG, WebP oder JPEG auf diesem Datenträger ausgewählt oder unter dem Cursor
strip-done = { $count ->
        [one] ein Bild
       *[other] { $count } Bilder
    } ohne Metadaten ({ $blocks } Blöcke entfernt), { $clean } hatten keine · Strg+Alt+Z stellt sie wieder her
undo-restore = { $count ->
        [one] Das Bild mit seinen Metadaten zurückholen:
       *[other] Die { $count } Bilder mit ihren Metadaten zurückholen:
    }
undo-done =
    { $count ->
        [one] ein Name wieder an seinem Platz
       *[other] { $count } Namen wieder an ihrem Platz
    }
slow-drives = die Laufwerksliste brauchte { $secs } s
slow-drives-late = die Laufwerksliste wartete { $secs } s: { $drives } antworteten nicht rechtzeitig und stehen ohne Angaben da — ein Netzlaufwerk ohne Server?
slow-listing = { $what } zu lesen brauchte { $secs } s
# editor search and undo (0.41)
btn-find-all = Alle Treffer
btn-replace-all = Alle ersetzen
btn-replace-one = Nächsten ersetzen
editor-replace-title = Ersetzen
editor-replace-style = Schreibweise beibehalten (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = In einem regulären Ausdruck stehen $1 und { "${name}" } für das, was eine Gruppe gefunden hat, $$ für ein Dollarzeichen
editor-matches-title = Zeilen mit `{ $query }`: { $count }
editor-matches-hint = Tippen zum Eingrenzen · Enter: zur Zeile · Esc: zurück
