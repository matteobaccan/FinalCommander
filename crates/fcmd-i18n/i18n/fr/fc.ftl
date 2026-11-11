key-help = Aide
key-menu = MenuUt
key-view = Voir
key-edit = Éditer
key-copy = Copier
key-move = Déplac
key-mkdir = CréDos
key-delete = Suppr
key-pulldown = Config
key-quit = Quitter
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Ajout
key-extract = Extrai
key-arccmd = ArcCmd
key-edit-new = Édit…
key-rename = Renom
key-group = Groupe
key-selup = SélHt
key-left = Gauche
key-right = Droite
key-sort-name = Nom
key-sort-ext = Exten
key-sort-write = Écrit
key-sort-size = Taille
key-sort-unsort = NonTri
key-sort-create = Créat
key-sort-access = Accès
key-sort = Tri
key-mklink = Lien
key-find = Cherch
key-history = Hist
key-folders = DossHs
key-plugins = Plugin

list-up = Haut
list-folder = Dossier
col-name = Nom
col-ext = Ext
col-size = Taille
col-allocated = Occupé
col-modified = Modifié
col-created = Créé
col-accessed = Accès
col-attr = Attr
col-owner = Propriétaire
col-group = Groupe
col-descr = Description
col-links = Lnk
col-git = Git
descr-title = Description de { $name }
descr-set = { $name } décrit
descr-cleared = description de { $name } retirée
descr-not-here = les descriptions vivent à côté d'un dossier de cette machine
col-date-time = Date       Heure
panel-list-count = { $count } éléments dans la liste
panel-empty = (vide)
panel-loading = lecture…

prompt-select-mask = Sélectionner les fichiers par masque :
prompt-deselect-mask = Désélectionner les fichiers par masque :
prompt-goto = Aller au répertoire :
prompt-filter = Masque ou partie d'un nom :
prompt-search = Recherche rapide :

dialog-error = Erreur
dialog-help-hint = Échap revient au sommaire · F7 cherche dans tous les chapitres · F1 ferme · ↑↓ PgPréc PgSuiv pour défiler
dialog-help-index = FinalCommander { $version } — Aide
dialog-help-hint-index = Entrée ouvre un chapitre · F7 cherche dans tous les chapitres · Échap ferme
help-search-prompt = Chercher dans l'aide
help-search-none = Aucune ligne de l'aide ne contient « { $query } »
help-search-title = « { $query } » dans l'aide : { $count } lignes
output-search-prompt = Chercher dans la sortie
output-search-empty = Rien n'a encore été affiché
output-search-found = Ligne { $line } sur { $lines }
output-search-wrapped = Ligne { $line } sur { $lines }, de nouveau depuis la plus récente
output-search-none = Aucune ligne de la sortie ne contient « { $query } »

msg-not-implemented = { $command } n'est pas encore implémenté dans cette version
msg-selected = { $count } éléments sélectionnés
msg-deselected = { $count } éléments désélectionnés
msg-mask-invalid = masque invalide : { $error }
msg-theme = thème : { $name }
msg-font-bigger = police : une plus grande demandée au terminal
msg-font-smaller = police : une plus petite demandée au terminal
msg-font-reset = police : celle du départ demandée au terminal
msg-sort = tri : { $key }{ $reverse }
msg-sort-groups-on = groupes de tri actifs ({ $count })
msg-sort-groups-off = groupes de tri désactivés ({ $count })
msg-sort-groups-none = aucun groupe de tri défini : voir [[sort_group]] dans la configuration
msg-view = vue : { $mode }
msg-hidden-on = fichiers cachés affichés
msg-hidden-off = fichiers cachés masqués
msg-no-history = plus d'historique
msg-tab-last = un panneau a toujours un onglet
msg-tab-end = l'onglet est déjà à ce bout de la rangée
tabs-title = Onglets
msg-tab-moved = onglet déplacé vers l’autre panneau
msg-tab-locked = cet onglet est fixé à son répertoire
msg-tab-locked-on = onglet fixé à son répertoire
msg-tab-locked-off = onglet libéré
msg-bookmark-set = raccourci { $digit } : { $path }
msg-bookmark-unset = le raccourci { $digit } n’est pas défini
msg-bookmark-added = { $path } gardé
msg-bookmark-removed = { $path } retiré
msg-bookmark-known = ce dossier est déjà dans la liste
msg-dirsize = { $path } : { $bytes } dans { $files } fichiers
msg-dirsize-many = mesure de { $count } dossiers…
usage-title = Occupation du disque : { $path } · { $size } · Tab : tout l'arbre en carte
usage-not-here = l'occupation d'un dossier se mesure sur un disque, pas dans une archive ni dans une liste de fichiers trouvés
msg-dirsize-no-folder = il n'y a aucun dossier à mesurer ici

prompt-copy-to = Copier { $what } vers :
prompt-move-to = Déplacer/renommer { $what } vers :
prompt-mkdir = Créer le répertoire :
prompt-tab-name = Nom de l’onglet :
prompt-bookmark-name = Nom du dossier :
what-selected = { $count } éléments sélectionnés
confirm-delete-title = Supprimer
confirm-delete-body =
    { $count ->
        [one] Supprimer { $name } ({ $bytes }) ?
       *[other] Supprimer { $count } éléments ({ $bytes }) ?
    }
prompt-link = Créer un lien vers { $name } sous :
link-created = { $name } créé
confirm-wipe-title = Effacement sécurisé
confirm-wipe-body =
    { $count ->
        [one] Écraser et supprimer { $name } ({ $bytes }) ?
       *[other] Écraser et supprimer { $count } éléments ({ $bytes }) ?
    }
confirm-wipe-honest = Les octets sont écrasés une fois, puis le fichier est supprimé. Sur un SSD, un système de fichiers copy-on-write ou avec des instantanés, des copies antérieures peuvent survivre — et le nom survit dans le répertoire et le journal.
job-copy = Copie
job-move = Déplacement
job-delete = Suppression
job-wipe = Effacement sécurisé
job-trash = Mise à la corbeille
job-replace = Remplacement
job-pack = Compression
job-rebuild = Écriture de l'archive

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Attributs du fichier
attr-count = { $count } éléments sélectionnés
attr-now = actuellement { $text } ({ $octal })
attr-owner-read = Le propriétaire peut lire
attr-owner-write = Le propriétaire peut écrire
attr-owner-exec = Le propriétaire peut exécuter
attr-group-read = Le groupe peut lire
attr-group-write = Le groupe peut écrire
attr-group-exec = Le groupe peut exécuter
attr-other-read = Tout le monde peut lire
attr-other-write = Tout le monde peut écrire
attr-other-exec = Tout le monde peut exécuter
attr-setuid = Exécuter en tant que propriétaire (setuid)
attr-setgid = Exécuter en tant que groupe (setgid)
attr-sticky = Seul le propriétaire peut supprimer (sticky)
attr-readonly = Lecture seule
attr-hidden = Caché
attr-system = Système
attr-archive = Archive
attr-temporary = Temporaire
attr-offline = Hors ligne
attr-indexed = L'indexeur peut le lire
attr-compressed = compressé
attr-encrypted = chiffré
attr-sparse = creux
attr-reparse = un point d'analyse
attr-also = également { $what }
attr-mtime = Modifié
attr-atime = Dernière lecture
attr-btime = Créé
attr-ctime = Enregistrement modifié
attr-btime-readonly = (la date de création ne se règle que sous Windows)
attr-utc = Heures en UTC
attr-owner = Propriétaire (utilisateur:groupe)
attr-owner-is = propriétaire { $who }
btn-attr-original = Original
btn-attr-current = Maintenant
btn-attr-blank = Vide
attr-recursive = Et tout ce qui est dans les dossiers
attr-bad-time = `{ $text }` n'est pas une date et heure (AAAA-MM-JJ HH:MM:SS)
attr-bad-owner = ici aucun utilisateur ni groupe ne s'appelle `{ $text }`
attr-in-archive = changer ceci réécrit l'archive, avec toutes les autres entrées telles quelles
job-attrs = Réglage des attributs

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Copier
copy-title-move = Renommer ou déplacer
copy-target = Vers
copy-conflict = S'il est déjà là
copy-conflict-ask = demander
copy-conflict-overwrite = remplacer
copy-conflict-newer = remplacer si la source est plus récente
copy-conflict-skip = passer
copy-conflict-rename = donner un nom libre à la copie
copy-conflict-append = ajouter à la fin
copy-only-newer = Seuls les fichiers plus récents que ceux présents
copy-filter = Seuls les noms qui correspondent
copy-follow-links = Copier ce que vise le lien, pas le lien
copy-keep-times = Garder les dates
copy-keep-mode = Garder les permissions
copy-verify = Relire chaque fichier et le comparer
copy-several = { $count } destinations, l'une après l'autre

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = un terminal avec { $shell }
term-hint = Ctrl+Alt+E : retour aux panneaux
term-ended = terminé — une touche ferme
job-mkdir = Création du répertoire
job-planning = analyse…
job-counters = { $files } / { $filestotal } fichiers · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · { $elapsed } écoulées · { $eta } restantes
job-rates-no-eta = { $speed }/s · { $elapsed } écoulées
job-hint = Esc annule
job-hint-queue = Échap annule · B arrière-plan
job-queued = { $kind } est en file, avec { $ahead } autres travaux non terminés · Alt+Shift+F5 les liste
job-background = { $kind } continue en arrière-plan · Alt+Shift+F5 liste les travaux
job-asks = { $kind } attend une réponse · Alt+Shift+F5
job-paused = { $kind } en pause
job-resumed = { $kind } reprend
job-retry-nothing = Rien à réessayer
job-still-waiting = Ce travail attend son tour
jobs-title = Travaux ({ $active } non terminés)
jobs-hint = Entrée afficher · Espace pause · Suppr arrêter · F5 réessayer · Ctrl+↑↓ ordre · Échap fermer
jobs-quit-title = Des travaux sont en cours
jobs-quit-body = Ces travaux s'arrêtent si le programme se termine maintenant :
job-state-running = en cours
job-state-paused = en pause
job-state-asking = demande
job-state-queued = en attente
job-state-held = retenu
job-state-done = terminé
job-state-failed = erreurs
job-state-cancelled = annulé
job-done = { $kind } : { $files } fichiers, { $bytes }
job-done-renamed = { $kind } : { $files } fichiers déplacés sans copie
job-done-skipped = , { $skipped } ignorés
job-done-verified = , { $verified } vérifiés
job-done-unverified = , { $unverified } impossibles à relire ici
job-done-errors = , { $errors } erreurs
job-cancelled = { $kind } annulée
job-errors-title = Erreurs
conflict-title = La destination existe
conflict-dst = destination  { $path }   { $bytes }
conflict-src = source       { $path }   { $bytes }
conflict-newer = la source est plus récente
conflict-older = la destination est plus récente
btn-overwrite = Remplacer
btn-overwrite-all = Tout
btn-skip = Ignorer
btn-skip-all = Tout ignorer
btn-retry = Réessayer
btn-move = Déplacer
btn-delete = Supprimer
btn-wipe = Effacer à fond
btn-copy = Copier
btn-discard = Abandonner
btn-kill = Terminer
btn-quit = Quitter
btn-play = Jouer
btn-run = Exécuter
btn-accept = Accepter
btn-cancel = Annuler
msg-nothing-to-do = rien de sélectionné
cmd-complete-nothing = rien pour compléter cela
complete-several = { $count } noms commencent ainsi
form-moving = les flèches déplacent la boîte · Entrée ou Échap la laisse là
form-moved = la boîte reste où vous l'avez mise
msg-trash-refused = La corbeille le refuse — Maj+Suppr le supprime définitivement.

# Checksums (SPEC 03 §9)
sum-title = Sommes de contrôle
sum-algorithm = Hachage
sum-one-file = Une seule liste pour tous
sum-name = Nom
sum-working = { $name } : { $done } sur { $total }
sum-verifying = vérification avec { $name }
sum-stopped = arrêté
sum-written = { $name } écrit
sum-no-name = la liste a besoin d’un nom
sum-local-only = une somme ne peut lire qu’un fichier de cette machine
sum-all-match = tous les { $count } correspondent
sum-verify-title = Sommes de contrôle
sum-verify-summary = { $ok } correspondent, { $bad } non
sum-differs = ne correspond pas
sum-missing = absent
sum-unreadable = illisible

# Synchronising two directories (SPEC 03 §10)
sync-title = Synchroniser
sync-between = { $left }  ↔  { $right }
sync-recursive = Aussi tout ce qui est dans les sous-répertoires
sync-both-ways = Dans les deux sens, le plus récent gagne
sync-delete-extra = Proposer de retirer ce que l’autre côté n’a pas
sync-by-content = Comparer les octets, pas la taille et la date
sync-filter = Seuls les noms qui correspondent
sync-walking = examine { $at } — { $pairs } jusqu’ici
sync-planned = { $copies } à copier ({ $bytes }), { $deletes } à retirer
sync-hint = Espace réponse suivante · ← → sens · = laisser · F2 faire · Échap annuler
sync-hint-walking = Échap arrête le parcours
sync-nothing-to-do = chaque ligne dit de ne rien changer
sync-local-only = une synchronisation ne peut lire et écrire que sur cette machine
sync-same-directory = les deux panneaux sont dans le même répertoire

# What to run for a kind of file (SPEC 06 §5)
assoc-none = rien n’indique comment ouvrir { $name }
usermenu-empty = Pas encore de menu utilisateur : écrivez-le dans { $path }
usermenu-last-gone = l'entrée choisie la dernière fois n'est plus dans ce menu
usermenu-bad-condition = Condition non comprise — { $problem }
assoc-desktop = Le confier au bureau
assoc-open-with = Ouvrir { $name } avec
assoc-no-pager = aucune association ni $PAGER n’indique avec quoi le lire
assoc-no-editor = aucune association ni $VISUAL n’indique avec quoi l’écrire

key-wrap = Retour
key-hex = Hex
key-text = Texte
key-dump = Dump
key-doc = Doc
key-search = Cherch
key-code = Encod
key-follow = Suivre
viewer-prompt-search = Chercher ({ $mode }) :
viewer-prompt-goto = Aller à l'offset, ou NN% :
viewer-not-found = `{ $query }` introuvable
viewer-follow-on = suit la fin du fichier
viewer-follow-off = suivi désactivé
viewer-codepage = encodage : { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = aucune position précédente
viewer-empty = (fichier vide)
viewer-not-markdown = ce n'est pas un fichier Markdown, ou il est trop grand à mettre en page
viewer-not-table = pas un fichier tabulaire, ou trop gros pour être lu en entier
viewer-base64-done = Décodé : { $bytes }
viewer-not-base64 = Ce fichier n’est pas du base64 : { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Désass
viewer-disasm-failed = impossible de désassembler { $name } : { $error }
viewer-decompile-running = { $tool } s'exécute sur { $name }…
viewer-decompile-none = rien ici ne décompile { $format } ; installez un outil et ajoutez une règle [[decompile]] dans fcmd.toml
arc-needs-program = cette archive est lue par `{ $program }`, qui n'est pas installé sur cette machine
viewer-decompile-failed = { $tool } : { $error }
viewer-blame-derived = git blame porte sur le fichier lui-même : revenir d’abord au fichier
viewer-derived = { $producer } de { $name }

key-save = Enreg
key-replace = Rempl
editor-prompt-goto = Aller à la ligne, ou ligne:colonne :
editor-prompt-save-as = Enregistrer sous :
editor-saved = { $name } : enregistré, { $bytes } octets
editor-nothing-to-save = { $name } n'a aucune modification
editor-not-found = `{ $query }` introuvable
editor-replaced = { $count } remplacements
editor-read-only = { $name } est en lecture seule
editor-readonly-on = lecture seule : les modifications sont refusées
editor-readonly-off = lecture seule désactivée
editor-no-undo = plus rien à annuler
editor-no-redo = rien à rétablir
editor-discard-title = Modifications non enregistrées
editor-discard-body = { $name } a des modifications non enregistrées. Quitter et les perdre ?
editor-changed-title = Modifié sur le disque
editor-changed-body = { $name } a changé depuis son ouverture. L'écraser ?
editor-reloaded = { $name } rechargé
editor-failed = { $name } : { $error }

# netbox (SPEC 04 §6)
net-sites-title = Connexions
net-sites-hint = Entrée connecte · n URL · Inser nouveau · F4 modifier · d déconnecte · Échap ferme
net-no-sites = Aucun site. `n` pour saisir une URL, ou ajoutez des blocs [[site]] à la configuration.
net-not-a-stored-session = cette ligne est une connexion ouverte, pas une session enregistrée
net-site-forgotten = { $name } n'est plus enregistrée
net-prompt-url = Se connecter à :
net-prompt-password = Mot de passe de { $user }@{ $host } :
net-prompt-passphrase = Phrase secrète de la clé de { $user }@{ $host } :
net-connecting = connexion à { $host }…
net-connected = connecté à { $host }
net-disconnected = { $host } déconnecté
net-not-connected = { $host } n'est pas connecté
net-local-panel = ce panneau est sur un système de fichiers local
net-nothing-pending = aucune connexion n'attend cela
net-hostkey-title = Clé d'hôte inconnue
net-hostkey-body = { $host } n'est pas dans known_hosts. Son empreinte :
net-hostkey-question = L'enregistrer et se connecter ?
net-hostkey-changed-title = La clé de l'hôte a CHANGÉ
net-hostkey-changed-body = { $host } a répondu avec une clé qui n'est pas celle enregistrée.
net-hostkey-changed-where = Celle enregistrée est dans { $path }, ligne { $line }.
net-hostkey-changed-what = Un serveur réinstallé ressemble exactement à quelqu'un au milieu. Rien n'est accepté ici : ouvrez le fichier, lisez la ligne, et retirez-la vous-même si vous savez pourquoi elle a changé.
btn-open-file = Ouvrir le fichier
net-fetching = récupération de { $name }…
net-viewing-copy = { $name } : copie locale, récupérée pour la visionneuse
net-too-big = { $name } dépasse { $limit } : augmentez net.fetch_limit_mb
net-editing = { $name } : vous modifiez une copie locale ; l'enregistrement la renvoie
net-writing-back = réécriture de { $name }…
net-written-back = { $name } : réécrit, { $bytes } octets
net-write-back-failed = réécriture échouée : { $error } — vos modifications sont dans { $local }
net-changed-title = Modifié sur le serveur
net-changed-body = { $name } a changé sur le serveur depuis sa récupération. L'écraser ?
confirm-delete-permanent = Ceci supprime définitivement : il n’y a pas de corbeille pour ceux-là.
confirm-trash-title = Mettre à la corbeille
confirm-trash-body =
    { $count ->
        [one] Mettre { $name } ({ $bytes }) à la corbeille ?
       *[other] Mettre { $count } éléments ({ $bytes }) à la corbeille ?
    }
confirm-trash-where = Ils vont là où le bureau garde les fichiers supprimés, et peuvent en être repris.

# ---------------------------------------------------------------- barre de menus (SPEC 10)
menu-left = &Gauche
menu-right = &Droite
menu-files = &Fichiers
menu-commands = &Commandes
menu-options = &Options
menu-brief = &Bref
menu-medium = &Moyen
menu-full = &Complet
menu-detailed = &Détaillé
menu-owners = &Propriétaires
menu-links = Liens
menu-alt-full = Complet, tailles entières
menu-descriptions = Descriptions
menu-long-descriptions = Descriptions longues
menu-wide = &Large
menu-info-panel = Pannea&u d'informations
menu-quick-view = &Aperçu rapide
menu-tree-panel = Panneau arbore&scent
menu-tab-new = &Nouvel onglet
menu-tab-close = &Fermer l’onglet
menu-tab-next = &Onglet suivant
menu-tab-prev = On&glet précédent
menu-tab-move = Déplacer l’onglet &vers l’autre panneau
menu-tab-lock = Fixer l’onglet &à son répertoire
menu-tab-name = Nommer l’onglet
menu-tab-list = Tous les onglets
menu-sort-modes = Modes de &tri
menu-hidden = Fichiers cachés &et système
menu-filter = Filtre de fic&hiers
menu-filter-sets = Filtres nommés
menu-panel-onoff = Panneau actif/&inactif
menu-reread = &Relire
menu-change-drive = Changer de dis&que
menu-view = &Voir
menu-edit = &Éditer
menu-edit-new = Éditer &un nouveau fichier
menu-copy = &Copier
menu-rename = &Renommer ou déplacer
menu-link = Créer un &lien
menu-multirename = Renommer plusieurs…
menu-multirename-undo = Annuler le dernier renommage multiple
menu-mkdir = Créer un &dossier
menu-delete = &Supprimer
menu-wipe = &Effacer définitivement
menu-checksum = Sommes de co&ntrôle…
menu-checksum-verify = Vérifier avec une liste
menu-split = Découper le fichier…
menu-combine = Réassembler les fichiers
menu-delete-permanent = Suppri&mer définitivement
menu-archive-add = &Ajouter à l'archive
menu-archive-extract = Extraire les &fichiers
menu-archive-commands = C&ommandes d'archive
menu-archive-test = &Tester l'archive
menu-attributes = Attri&buts du fichier
menu-apply-command = Appli&quer une commande
menu-replace-in-files = Remplacer dans les fic&hiers…
menu-select-group = Sélectionner un &groupe
menu-select-ext = Sélectionner &par cette extension
menu-unselect-ext = Désélectionner par cette e&xtension
menu-unselect-group = Désélectionner un groupe
menu-invert-selection = &Inverser la sélection
menu-select-all = Tout sélectionner
menu-restore-selection = Restaurer la sélection
menu-dirsize = Taille du dossier
menu-find-file = &Chercher un fichier
menu-history = &Historique des commandes
menu-view-history = Historique &des vues et éditions
menu-folders-history = H&istorique des dossiers
menu-usage = Occupation du dis&que
menu-jobs = Liste des tra&vaux
menu-branch = Vue en arborescence plate
menu-bookmarks = Dossiers favoris
menu-swap-panels = &Échanger les panneaux
menu-sync-panel = &Synchroniser l'autre panneau
menu-panels-onoff = &Panneaux actifs/inactifs
menu-compare-folders = Comparer &les dossiers
menu-sync-folders = S&ynchroniser les dossiers
menu-compare-contents = Comparer par co&ntenu
menu-panelize = Panneau depuis la sortie d'une commande
menu-view-filtered = Voir à travers une commande
menu-learn-keys = Apprendre les touches
menu-tunnel = &Ouvrir un tunnel…
menu-tunnels = &Tunnels…
tunnel-title = Tunnel : [local:]hôte:port
tunnel-list-title = Tunnels
tunnel-none = aucun tunnel ouvert
tunnel-open = tunnel ouvert : { $what }
tunnel-closed = tunnel fermé
tunnel-failed = { $what } : { $why }
tunnel-needs-connection = un tunnel passe par une connexion : ouvrez-en une avec Alt+N
menu-connections = Conne&xions
menu-disconnect = Déconnecter
menu-user-menu = &Menu utilisateur
menu-associations = &Associations de fichiers
menu-folder-shortcuts = &Raccourcis de dossiers
menu-plugin-commands = Commandes des plu&gins
menu-screens-list = Liste des écrans
menu-macro-record = &Enregistrer une macro
menu-macro-list = Macros
menu-task-list = Liste des t&âches
menu-system-settings = &Réglages système
menu-panel-settings = Réglages &des panneaux
menu-interface-settings = Réglages de &l'interface
menu-languages = La&ngues
menu-themes = &Thèmes
menu-confirmations = C&onfirmations
menu-viewer-settings = Réglages de la &visionneuse
menu-editor-settings = R&églages de l'éditeur
menu-codepages = &Pages de codes
menu-sort-groups = &Groupes de tri
menu-mask-groups = Masq&ues de fichiers
menu-settings-all = Tou&s les réglages
menu-highlighting = &Mise en évidence des fichiers
menu-plugins-config = Con&figuration des plugins
menu-help = &Aide
menu-save-setup = &Enregistrer les réglages
menu-quit = &Quitter
opt-natural-sort = Ordre naturel des nombres
opt-allow-reverse = Autoriser le tri inverse
opt-select-folders = Les masques sélectionnent aussi les dossiers
cmd-no-network-name = Un chemin sur ce disque n'a pas de nom réseau
opt-trash = Utiliser la corbeille du système
opt-system-copy = Utiliser la copie du système
opt-save-history = Conserver l'historique entre les sessions
opt-history-lifetime = Durée de vie de l'historique, en jours
opt-forever = pour toujours
opt-fetch-limit = Taille maximale des fichiers distants
opt-keepalive = Keep-alive SSH
opt-off = désactivé
opt-show-hidden = Afficher les fichiers cachés et système
opt-column-titles = Afficher les titres de colonnes
opt-status-line = Afficher la ligne d'état
opt-scrollbar = Afficher la barre de défilement
opt-free-space = Afficher l'espace libre
opt-auto-refresh = Actualiser le panneau automatiquement
opt-dirs-first = Dossiers d'abord
opt-selected-first = Fichiers sélectionnés d'abord
opt-reverse = Ordre inverse
opt-view = Vue du panneau
opt-sort = Trier par
opt-quick-search = Recherche rapide
opt-quick-alt = avec Alt
opt-quick-plain = par lettres
opt-key-bar = Afficher la barre de touches
opt-status-bar = Afficher la barre d'état
opt-syntax = Colorer les langages reconnus
opt-markdown = Markdown mis en page pour la lecture
opt-table = Ouvrir un .csv ou un .jsonl comme un tableau
opt-keymap = Carte des touches
opt-clock = Horloge dans le coin
opt-mouse = Prise en charge de la souris
opt-clipboard = Presse-papiers via le terminal (OSC 52)
opt-command-screen = Montrer la sortie de la commande pendant qu'elle tourne
opt-menu-always = Toujours afficher la barre de menus
opt-save-exit = Enregistrer la configuration en quittant
opt-confirm-delete = Suppression
opt-confirm-overwrite = Écrasement
opt-confirm-exit = Sortie
opt-confirm-editor = Quitter l'éditeur avec des modifications non enregistrées
opt-applied = réglages appliqués
opt-saved = enregistré dans { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Pas un oui-ou-non : Entrée saisit une valeur
opt-settings-hint = ↑↓ choisir · tapez pour filtrer · Entrée ou F4 valeur · Espace inverser · Suppr défaut · Échap fermer
opt-language-set = langue : { $name }
sort-name = Nom
sort-ext = Extension
sort-size = Taille
sort-mtime = Date
sort-btime = Création
sort-atime = Dernier accès
sort-descr = Description
sort-owner = Propriétaire
sort-git = État git
key-sort-descr = Descr
key-sort-owner = Propr
sort-none = Non trié
sort-set = tri : { $name }
btn-ok = OK
pick-hint = ↑↓ choisir · tapez pour filtrer · Entrée appliquer · Échap fermer
pick-empty = (rien à choisir)
# ------------------------------------------------- les trois historiques (SPEC 06 §6)
hist-pinned = { $line } est épinglée · plus rien ne la chasse
hist-unpinned = { $line } n’est plus épinglée
hist-is-pinned = { $line } est épinglée · Inser la libère d’abord
hist-clear-body = Vider cet historique de tout ce qui n’est pas épinglé ?
hist-cleared = l’historique est vide, sauf ce qui est épinglé
history-hint = ↑↓ choisir · Entrée appliquer · Inser épingler · Maj+Suppr une · Suppr toutes · Échap fermer


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Nouvelle connexion
site-edit = Modifier la connexion
site-name = Nom
site-host = Hôte
site-port = Port
site-user = Utilisateur
link-title = Créer un lien
link-name = Le lien
link-kind = Type
link-kind-relative = symbolique, relatif — survit au déplacement de l’arborescence
link-kind-absolute = symbolique, absolu — survit au déplacement du lien
link-kind-hard = physique — un second nom pour le même fichier
menu-colors = &Couleurs…
palette-title = Couleurs de { $theme }
palette-hint = Entrée une couleur · F2 enregistrer · Échap fermer
palette-ask = Une couleur pour { $key }
palette-bad-color = { $value } n’est pas une couleur (un nom, 0–255 ou #rrggbb)
palette-saved = les couleurs ont été écrites dans le thème { $theme }
palette-unsaved = les couleurs n’ont changé qu’à l’écran — F2 les écrit
opt-viewer-status-line = Afficher la ligne du haut (nom et position)
opt-viewer-scrollbar = Afficher une barre de défilement
opt-viewer-key-bar = Afficher la barre des touches
opt-viewer-wrap = Lignes longues
opt-viewer-wrap-off = ne pas couper
opt-viewer-wrap-char = couper au bord
opt-viewer-wrap-word = couper au mot
opt-viewer-tab-width = Largeur de tabulation
opt-viewer-remember = Retenir la position et l'encodage de chaque fichier
opt-editor-tabs = Tabulations
opt-editor-tabs-keep = les laisser telles quelles
opt-editor-tabs-typed = convertir celles que l’on tape
opt-editor-tabs-all = les convertir toutes à l’ouverture
opt-editor-tab-width = Largeur de tabulation
opt-editor-auto-indent = Garder l’indentation de la ligne précédente
opt-editor-whitespace = Montrer espaces et tabulations
opt-editor-cursor-after = Le curseur se place après le texte trouvé
opt-editor-beyond-eol = Le curseur peut dépasser la fin de la ligne
opt-editor-trim-trailing = Retirer les blancs de fin de ligne à l'enregistrement
editor-not-a-character = cette touche n’a aucun caractère à insérer
paste-one-line =
    { $count ->
        [one] seule la première ligne a été collée · une ligne de plus laissée de côté
       *[other] seule la première ligne a été collée · { $count } lignes de plus laissées de côté
    }
clip-names =
    { $count ->
        [one] un nom dans le presse-papiers
       *[other] { $count } noms dans le presse-papiers
    }
clip-paths =
    { $count ->
        [one] un chemin dans le presse-papiers
       *[other] { $count } chemins dans le presse-papiers
    }
clip-line = la ligne de commande est dans le presse-papiers
clip-selection = la sélection est dans le presse-papiers
clip-off = le presse-papiers est désactivé — `ui.clipboard` dans les réglages
history-gone = { $name } n’est plus là
hl-new = Nouveau groupe de couleur
hl-edit = Groupe de couleur
# ------------------------------------------------- les différences (SPEC 05 §3)
diff-title = Différences
menu-diff = Comparer deux &fichiers
diff-needs-two = Mettre le curseur sur un fichier dans chaque panneau
diff-not-here = { $name } n'est pas un fichier sur ce disque
diff-too-big = { $name } est plus grand que { $most }
diff-same = Les deux fichiers sont identiques
diff-where = bloc { $at } sur { $count } · -{ $removed } +{ $added } lignes
diff-no-next = La dernière différence
diff-no-previous = La première différence
diff-hint = Alt+↑↓ blocs · Tab côté · F5 prendre · F2 écrire · W espaces · C casse · E fins de ligne · Échap fermer
diff-hint-git = Alt+↑↓ blocs · S indexer le bloc · Tab côté · F5 prendre · F2 écrire · Échap fermer
diff-hint-git-staged = Alt+↑↓ blocs · S désindexer le bloc · Tab côté · F5 prendre · F2 écrire · Échap fermer
diff-stage-edited = le diff a des fusions non écrites : F2 d’abord, puis S
diff-merged-left = Le fichier de gauche a pris cette différence
diff-merged-right = Le fichier de droite a pris cette différence
diff-no-hunk-here = Aucune différence en haut de l'écran — Alt+↓ en amène une ici
diff-nothing-to-write = Aucune différence n'a été prise
diff-written = { $count } écrits
diff-unwritten = Une différence prise n'est pas écrite — F2 l'écrit, Échap de nouveau l'abandonne
diff-ignore-space-on = Espaces ignorés
diff-ignore-space-off = Les espaces comptent à nouveau
diff-ignore-case-on = Casse ignorée
diff-ignore-case-off = La casse compte à nouveau
diff-ignore-eol-on = Fins de ligne ignorées
diff-ignore-eol-off = Les fins de ligne comptent à nouveau

grab-hint = ↑↓←→ déplacer · Maj+↑↓←→ marquer · Ctrl+A tout · Entrée copier · Échap fermer
grab-copied = { $rows } lignes de l'écran copiées

# ------------------------------------------------- les pages de codes (SPEC 05 §1.12)
cp-title = Pages de codes
cp-name-title = Comment appeler cette page de codes
cp-set = Lu comme { $name }
menu-panel-names = Page de codes des noms…
names-title = Lire les noms comme
names-system = Comme le système les donne
names-set = Noms lus comme { $name }
names-system-set = Noms comme le système les donne
names-not-here = Un nom sur ce système n'est pas des octets, il n'y a donc pas de seconde lecture
cp-marked = { $name } fait partie de celles que F8 parcourt
cp-unmarked = { $name } ne fait plus partie de celles que F8 parcourt
cp-hint = ↑↓ choisir · Entrée lire ainsi · Inser marquer pour F8 · F4 renommer · Échap fermer

# ------------------------------------------------- les groupes de masques (SPEC 02 §5b)
mg-title = Groupe de masques
mg-name = Nom
mg-mask = Masques
mg-hint = Inser ajoute un groupe, Suppr le retire, F4 modifie · un `·` marque les trois intégrés
mg-saved = <{ $name }> est en vigueur ; Enregistrer les réglages l’écrit
mg-removed = <{ $name }> n'est plus là
mg-bad = <{ $name }> : { $problem }
mg-needs-name = un nom sans espace, virgule ni chevron
mg-needs-mask = un groupe qui ne représente rien ne correspond à rien
mg-built-in = <{ $name }> est l'un des trois intégrés · F4 lui donne d'autres masques

sg-title = Groupe de tri
sg-name = Nom
sg-mask = Masques
sg-place = Où
sg-place-front = devant les autres
sg-place-back = derrière les autres
sg-hint = Inser ajoute une bande, Suppr l'enlève, F4 la modifie, Ctrl+↑↓ la déplace
sg-saved = La bande est en vigueur ; Enregistrer la configuration l'écrit
sg-removed = { $name } n'est plus là
sg-bad = { $name } : { $problem }
sg-needs-mask = une bande sans masque ne contiendrait rien
hl-name = Nom
hl-mask = Masque ou masques
hl-kind = S’applique à
hl-kind-both = fichiers et dossiers
hl-kind-file = fichiers seulement
hl-kind-dir = dossiers seulement
hl-size = Taille (1M-10M, -4K, 1G-)
hl-age = Âge en jours (<=1, >=30, 7-30)
hl-date-field = L’âge porte sur
hl-attrs = Attributs (h r x s b, - pour « ne doit pas »)
hl-fg = Couleur (#d7875f, red, file.archive)
hl-bg = Fond, écrit de la même façon
hl-hint = ↑↓ choisir · Enter/F4 modifier · Ins nouveau · Del supprimer · Ctrl+↑↓ déplacer · Esc fermer
hl-saved = le groupe est actif · Options → Enregistrer les réglages l’écrit dans le fichier
hl-removed = { $name } supprimé · Options → Enregistrer les réglages l’écrit dans le fichier
hl-bad = { $name }: { $problem }
filter-off = Pas de filtre : tous les noms
filter-hint = ↑↓ choisir · Entrée utiliser · F4 modifier · Inser nouveau · Suppr retirer · Échap fermer
filter-new = Nouveau filtre
filter-edit = Filtre
filter-name-hint = avec un nom il reste dans le menu des filtres ; sans nom il sert une fois
filter-applied = filtre actif : { $name }
filter-cleared = filtre retiré
filter-saved = { $name } est sur le panneau et dans le menu des filtres · Options → Enregistrer les réglages l'écrit dans le fichier
filter-removed = { $name } n'est plus là · Options → Enregistrer les réglages l'écrit dans le fichier
editor-no-block = rien n’est sélectionné
editor-unique = { $count } lignes répétées retirées
editor-no-bracket = pas de parenthèse ici, ou aucune ne lui correspond
editor-tabs-expanded = tabulations converties sur { $lines } lignes — l’enregistrement l’écrira
site-protocol = Protocole
site-protocol-sftp = SFTP (sur SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP avec TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV sur HTTPS
site-auth = Authentification
site-auth-agent = agent SSH
site-auth-key = fichier de clé
site-auth-ask = demander le mot de passe
site-key-file = Fichier de clé
site-start-dir = Dossier distant
site-group = Groupe
site-keepalive = Keep-alive (s)
site-compress = Compression
site-page-session = Session
site-page-environment = Environnement
site-page-directories = Dossiers
site-page-sftp = SFTP
site-password-note = Aucun mot de passe n'est conservé : il est demandé à la connexion.
site-env-eol = Convertir les fins de ligne (pas encore)
site-env-utf8 = Forcer les noms en UTF-8 (pas encore)
site-jump = Hôte de rebond
site-jump-note = On y arrive en passant par cet hôte : hôte, utilisateur@hôte ou utilisateur@hôte:port. Vide signifie directement au serveur.
site-sftp-ciphers = Choisir les algorithmes de chiffrement (pas encore)
btn-connect = Connecter
site-needs-host = une connexion a besoin d'un hôte
site-needs-key = choisissez le fichier de clé, ou une autre authentification
site-bad-port = « { $port } » n'est pas un nombre
site-stored = connexion enregistrée : { $name } (Shift+F9 pour la garder)
# ------------------------------------------------- panneau structure (SPEC 05 §1.8)
key-struct = Struct
key-markdown = MD
key-table = Table
key-filter = Filtre
key-decompile = Décomp
key-next = Suiv
key-prev = Préc
key-codepages = PageC
key-goto = AllerA
key-to-panel = Panneau
key-blame = Blame
viewer-structure = Structure
viewer-structure-hint = ↑↓ ligne · Entrée aller à · Tab contenu · F11 ferme
viewer-structure-hint-tab = Tab amène les touches ici
viewer-no-structure = rien à décoder : le panneau ne connaît pas ce format
viewer-no-target = cette ligne ne pointe nulle part dans le fichier
viewer-form-opened = La fiche, comme le texte que Delphi en montre. F4 revient aux octets.
viewer-metadata-opened = Le texte entier, mis en page pour le lire. F4 revient aux octets.
viewer-went-to = positionné à { $offset }
viewer-in-panel = le panneau est sur { $name }
viewer-not-in-panel = { $name } n'est pas dans ce panneau
viewer-no-more-files = plus de fichiers dans le panneau
editor-unsaved-for-viewer = enregistrez d'abord : la visionneuse lit le fichier sur le disque
# ------------------------------------------------- ligne de commande (SPEC 06 §7)
cmd-running = en cours : { $command }
prompt-panelize = Commande dont la sortie nomme les fichiers
prompt-viewer-filter = Commande dont lire la sortie au lieu du fichier
learn-title = Quelles touches ce terminal envoie
learn-hint = Appuyez sur les touches. Une case allumée est arrivée ; Esc deux fois pour sortir.
learn-last = dernière : { $key } — { $command }
learn-unbound = liée à rien
recode-title = Réécrire en
recode-what = { $count } fichiers
recode-from = Ce qu'ils sont maintenant
recode-to = Les écrire en
recode-auto = le déduire fichier par fichier
recode-eol = Fins de ligne
recode-eol-keep = les laisser telles quelles
recode-working = réécriture de { $count } fichiers
recode-done = { $changed } écrits, { $same } déjà ainsi, { $failed } refusés
menu-recode = Réécrire en…
panelize-title = { $command }
panelize-done = { $count } fichiers, { $dropped } lignes ignorées
panelize-nothing = { $command } : aucune ligne ne nomme un fichier existant
cmd-interrupted = interruption envoyée à la commande
cmd-done = { $command } terminé
cmd-failed = { $command } s'est terminé avec { $code }
cmd-busy = une commande est déjà en cours
prompt-apply = Commande à exécuter sur chacun des { $count } fichiers :
cmd-apply-no-ask = une commande qui demande une valeur ne peut pas être appliquée fichier par fichier
cmd-apply-progress = { $command }  ({ $done } sur { $total })
cmd-apply-stopped = arrêté après { $done } sur { $total } : { $command } a échoué
cmd-not-remote = une commande a besoin d'un dossier local ; ce panneau est sur un serveur
cmd-no-history = l'historique des commandes est vide
cmd-no-previous = nulle part où revenir
cmd-no-such-dir = { $path } : dossier introuvable
cmd-no-such-path = { $path } : fichier ou dossier introuvable
cmd-no-selection = rien n'est sélectionné
cmd-condition-false = non exécuté : { $command }
cmd-screen-cleared = écran effacé
cmd-stack-empty = la pile de dossiers est vide
cmd-stack-cleared = pile de dossiers vidée
cmd-pushed = { $count } dossiers dans la pile
cmd-environment = environnement
cmd-set = { $name } = { $value }
cmd-not-set = { $name } n'est pas définie
user-screen-title = Sortie — { $lines } lignes, à partir de la ligne { $at }
user-screen-empty = (rien n'a encore été exécuté)
viewer-no-branch = cette ligne n'a pas d'adresse à suivre
viewer-no-back = rien où revenir

# ------------------------------------------------- recherche de fichiers (SPEC 06 §1)
folders-title = Trouver un dossier
folders-reading = Lecture des dossiers…
folders-read = { $count } dossiers
folders-capped = { $count } dossiers, jusqu'à la limite
folders-local-only = la liste des dossiers est celle de cette machine ; un serveur a son historique et les favoris
find-title = Chercher un fichier
find-mask = Masque ou masques
find-size = Taille (100M, 1M-1G, -4K)
find-text = Contenant le texte
find-case = Respecter la casse
find-words = Mots entiers uniquement
find-hex = Le texte ci-dessus est un motif hexadécimal
find-not = Fichiers qui ne le contiennent PAS
find-scope = Chercher dans
find-scope-panel = ce dossier
find-scope-tree = ce dossier et en dessous
find-scope-selection = les dossiers sélectionnés
find-scope-root = depuis la racine de ce disque
find-scope-other = l'autre panneau et en dessous
find-links = Suivre les liens symboliques
find-dirs = Les dossiers aussi dans les résultats
find-archives = Dans les archives
find-results = { $count } trouvés · { $files } fichiers et { $dirs } dossiers examinés
find-done = { $count } trouvés dans { $files } fichiers · { $elapsed }
find-none = aucun résultat
find-stopped = arrêté après { $files } fichiers
find-hint = ↑↓ choisir · Entrée aller · F3 voir · F4 éditer · F5 dans le panneau · Échap arrêter
find-hint-done = ↑↓ choisir · Entrée aller · F3 voir · F4 éditer · F5 dans le panneau · Échap fermer
find-panelised = { $count } résultats dans le panneau
find-needs-mask = un masque est nécessaire : `*` veut dire tous les fichiers
find-bad-size = `{ $text }` n’est pas une taille — { $error }

# ------------------------------------------------- replace across files
replace-title = Remplacer dans les fichiers
replace-needle = Rechercher
replace-with = Remplacer par
replace-case = Respecter la casse
replace-words = Mots entiers uniquement
replace-pattern = Le texte ci-dessus est une expression régulière
replace-hex = Les deux sont des octets hexadécimaux
replace-scope = { $count } fichiers sélectionnés
replace-needs-needle = il n'y a rien à chercher
replace-confirm-title = Remplacer
replace-confirm-body = { $count } occurrences dans { $files } fichiers changeraient.
replace-nothing = rien à remplacer : aucun fichier ne changerait
replace-done = { $count } occurrences remplacées dans { $files } fichiers

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = { $name } retourne dans l'archive
arc-move-out = pour sortir quelque chose d'une archive, copiez-le avec F5
arc-add-files-only = un par un, seuls les fichiers entrent ; Shift+F1 y met un répertoire entier

rename-title = Renommer plusieurs
rename-name = Nom
rename-ext = Extension
rename-search = Rechercher
rename-replace = Remplacer par
rename-regex = Expression régulière ($1 dans le remplacement)
rename-case = Lettres
rename-case-same = telles quelles
rename-case-lower = minuscules
rename-case-upper = MAJUSCULES
rename-case-title = Chaque Mot En Capitale
rename-case-sentence = Première lettre seulement
rename-counter = Compteur : début, pas, chiffres
rename-fields = [N] nom  [E] extension  [N1-3] partie  [C] compteur  [d] [t] date, heure  [P] dossier
rename-not-in-archive = plusieurs fichiers se renomment dans un dossier, pas dans une archive : là, F6 renomme une entrée
rename-bad-counter = le compteur, ce sont trois entiers, début, pas, chiffres : { $counter } n'en est pas
rename-cannot = impossible de renommer : { $why }
rename-problem-empty = il ne reste aucun nom
rename-problem-invalid = ce n'est pas un nom qu'un fichier peut avoir
rename-problem-twice = un autre fichier prend aussi ce nom
rename-problem-exists = un fichier de ce nom est déjà là
rename-more = … et { $count } de plus
rename-blocked =
    { $count ->
        [one] un fichier ne peut pas prendre son nouveau nom : rien n'est renommé tant qu'il ne le peut pas
       *[other] { $count } fichiers ne peuvent pas prendre leur nouveau nom : rien n'est renommé tant qu'ils ne le peuvent pas
    }
rename-summary = { $count } sur { $total } changent
rename-nothing = aucun nom ne change
rename-nothing-to-undo = aucun renommage multiple à annuler
rename-done =
    { $count ->
        [one] un fichier renommé · annulable depuis le menu Fichiers
       *[other] { $count } fichiers renommés · annulables depuis le menu Fichiers
    }
rename-undone =
    { $count ->
        [one] un nom remis
       *[other] { $count } noms remis
    }
pack-title = Ajouter à l'archive
pack-name = Archive
pack-format = Format
pack-level = Compression
pack-solid = Blocs solides (7z : plus petit, plus lent pour un fichier)
pack-delete-after = Supprimer les fichiers après l'archivage
pack-level-normal = normale
pack-level-fast = rapide
pack-level-best = maximale
pack-scope = { $count } fichiers sélectionnés
pack-needs-name = l'archive a besoin d'un nom
pack-format-read-only = { $format } se lit ici, ne s'écrit pas : choisissez un autre format ou un autre nom
pack-single-one-file = un { $format } contient un seul fichier : choisissez-en un, ou prenez { $tar } pour un dossier ou plusieurs fichiers
unpack-title = Extraire de l'archive
unpack-to = Extraire vers
unpack-scope = depuis { $name }
unpack-not-an-archive = pas une archive que nous savons lire

# ── les panneaux particuliers (SPEC 02 §7) ──────────────────────────────────────────────
panel-quick-view = Aperçu rapide
panel-info = Informations
panel-tree = Arborescence
compare-done = { $left } à gauche, { $right } à droite diffèrent
cmp-subdirs = Entrer dans les sous-dossiers
cmp-depth = Jusqu'à quelle profondeur (0 = tout en bas)
cmp-trees-title = Ce qui diffère
cmp-trees-working = comparaison des arbres : { $seen } fichiers, { $read } lus
cmp-trees-done = { $differ } diffèrent, { $left } seulement à gauche, { $right } seulement à droite, sur { $pairs } paires
cmp-trees-same = les deux arbres sont identiques ({ $pairs } paires, { $same } vérifiées)
menu-duplicates = Trouver les doublons
dup-title = Fichiers en double
dup-working = recherche des doublons : { $seen } fichiers, { $hashed } lus
dup-done = { $groups } groupes de copies, { $files } fichiers, { $wasted } gaspillés
dup-none = rien ici n'est la copie d'autre chose ({ $seen } fichiers)
dup-stopped = arrêté après { $seen } fichiers
dup-partial = arrêté avant la fin
cmp-title = Comparer
cmp-case = Comparer les noms avec leur casse
cmp-size = Une taille différente est une différence
cmp-time = Une date différente est une différence
cmp-seconds = Des dates à tant de secondes près sont le même instant
cmp-contents = Lire les octets
cmp-when-time = Ne les lire que si les dates diffèrent
cmp-fragment = Ne lire que les premiers Kio (0 = tout)
cmp-ignore-eol = Les fins de ligne seules ne sont pas une différence
cmp-selected = Seulement ce qui est sélectionné
cmp-stop = S'arrêter à la première différence
cmp-newer = Ne sélectionner que la copie la plus récente
menu-compare-setup = Comparer avec options
compare-contents-eol-ignored = { $count } ne diffèrent que par les fins de ligne, ignorés
compare-contents-trusted = { $count } considérés identiques sans les lire
compare-contents-working = Comparaison du contenu : { $done } sur { $total }
compare-contents-done = { $differ } paires sur { $pairs } diffèrent
compare-contents-same = les { $pairs } paires sont identiques
compare-contents-none = aucun fichier ne porte le même nom des deux côtés
compare-contents-local = comparer par contenu demande les deux panneaux sur un système de fichiers local
compare-contents-eol = { $count } seulement dans les fins de ligne
compare-contents-only = { $count } d'un seul côté
compare-contents-unread = { $count } illisibles
compare-contents-stopped = comparaison interrompue
quick-view-directory = { $name } est un répertoire — F3 le mesure
quick-view-not-here = les octets ne sont pas ici : F3 les extrait d'abord
info-path = Chemin
info-kind = Type
info-kind-local = un répertoire sur cette machine
info-kind-remote = sur { $host }
info-kind-archive = dans { $name }
info-kind-list = une liste de fichiers
info-device = Disque
info-folder = Dossier
info-device-value = { $total }, { $free } libres, clusters de { $cluster }
info-slack = Perte
info-filesystem = Système de fichiers
info-mount = Monté
info-git = Dépôt
info-git-clean = comme dans l'index
info-git-differing = { $count } sur { $tracked } diffèrent de l'index
info-git-uncounted = non comptés
info-git-merge = fusion en cours
info-git-rebase = rebase en cours
info-git-cherry-pick = cherry-pick en cours
info-git-revert = revert en cours
info-git-bisect = bisect en cours
info-memory = Mémoire
info-memory-value = { $free } libres sur { $total }
info-memory-total = { $total }
info-contents = Contient
info-contents-value = { $files } fichiers, { $dirs } répertoires, { $bytes }
info-selected = Sélection
info-selected-value = { $count } fichiers, { $bytes }
info-cursor = Curseur
info-size = Taille
info-modified = Modifié
info-description = Description
key-mark = Marque
key-asm = Asm
key-block = Bloc
key-screens = Écrans
key-site-edit = ModSess
key-site-new = NouvSes
key-site-forget = SuprSes
key-ai-edit = Modif
key-ai-new = Nouveau
key-ai-remove = Retirer
key-ai-forget = Retirer
key-ai-threads = Historique
hex-needs-a-file = ce tampon n'a pas de fichier d'où lire des octets
hex-save-first = enregistrez d'abord : les deux éditeurs ne peuvent pas se passer les modifications

hex-block-title = Bloc
hex-op-write = Écrire les { $size } dans un fichier
hex-op-read = Lire un fichier à { $at }
hex-op-insert = Insérer des octets nuls à { $at }
hex-op-copy = Copier des octets d'ailleurs vers { $at }
hex-op-move = Déplacer des octets d'ailleurs vers { $at }
hex-op-fill = Remplir les { $size } avec un octet
hex-op-xor = Ou-exclusif des { $size } avec une clé
hex-op-delete = Supprimer les { $size } — { $was } devient { $becomes }
hex-write-title = Écrire le bloc dans
hex-read-title = Quel fichier lire
hex-fill-title = Remplir avec quel octet (hex)
hex-xor-title = Ou-exclusif avec quelle clé (hex)
hex-insert-title = Combien d'octets nuls
hex-copy-title = Copier depuis où — position et longueur (hex)
hex-move-title = Déplacer depuis où — position et longueur (hex)
hex-not-hex = ce ne sont pas des chiffres hexadécimaux
hex-not-a-count = ce n'est pas un nombre d'octets
hex-not-a-range = ce n'est pas une position suivie d'une longueur en hexadécimal
hex-range-outside = cette plage est en dehors du fichier ({ $size })
hex-move-into-itself = un bloc ne peut pas être déplacé en lui-même
hex-filled = { $size } remplis
hex-xored = ou-exclusif sur { $size } — ce n'est pas du chiffrement
hex-written = { $size } écrits dans { $name }
hex-read-done = { $size } lus depuis { $name }
hex-inserted = { $size } de zéros insérés
hex-deleted = { $size } supprimés
hex-device = { $name } : un périphérique de { $size }, ouvert en lecture seule
asm-title = Assembler à { $at } ({ $arch })
asm-no-arch = { $arch } n'est pas une architecture que nous savons assembler
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } octets de plus)
screens-title = Écrans
screens-panels = Panneaux
quit-title = Quitter FinalCommander
quit-body = Quitter ?
screens-quit-title = Modifications non enregistrées
screens-quit-body = Ces écrans portent des modifications non écrites :
macro-recording = Enregistrement d'une macro · Ctrl+. arrête
macro-empty = rien n'a été enregistré
macro-dropped = enregistrement abandonné
macro-key-title = Quelle touche
macro-key-body = { $steps } étapes enregistrées. Appuyez sur la touche de cette macro, ou Échap pour l'abandonner.
macro-describe = À quoi sert la macro sur { $key }
macro-saved = macro sur { $key } enregistrée, { $steps } étapes
macro-none = aucune macro enregistrée · Ctrl+. en enregistre une
macro-list-title = Macros
macro-play-title = Comment l'exécuter
macro-play-once = Une fois, depuis le curseur
macro-play-each = Une fois par fichier sélectionné
macro-sure-title = Exécuter cette macro
macro-sure-body = La macro sur { $key } copie, déplace, supprime ou écrase. L'exécuter ?
macro-too-deep = une macro ne peut pas s'appeler indéfiniment
plugin-list-title = Commandes des greffons
plugin-none-title = Aucun greffon installé
plugin-none-open = Ouvrir le dossier des greffons ({ $dir })
plugin-none-help = Comment marchent les greffons, et où en trouver
plugin-help-heading = Plugins
plugin-why-off = désactivé dans Options → Configuration des greffons
plugin-why-panel = seulement dans les panneaux
plugin-why-viewer = seulement dans la visionneuse
plugin-why-editor = seulement dans l'éditeur
plugin-why-local = il faut un dossier sur les disques de cette machine
plugin-why-files = il faut un fichier sous le curseur ou sélectionné
plugin-why-masks = seulement pour { $masks }
plugin-why-bad-mask = son masque est faux : { $error }
plugin-why-here = il faut { $names } dans ce dossier
plugin-why-inside = il faut { $names } ici ou dans un dossier au-dessus
plugin-cannot = { $label } : { $why }
plugin-running = { $label }…
plugin-on-selected = sur { $count } fichier(s) sélectionné(s)
plugin-failed = { $id } : { $why }
plugin-unknown-command = un greffon a demandé « { $id } », qui n'est pas une commande
hex-copied = { $size } copiés depuis { $from }
hex-moved = { $size } déplacés depuis { $from }
menu-calculator = Calc&ulatrice
menu-terminal = Terminal
calc-title = Calculatrice
calc-decimal = décimal
calc-hex = hexa
calc-octal = octal
calc-binary = binaire
calc-characters = caractères

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Commandes d'archive
arc-password = Mot de passe pour { $name }
arc-op-test = Tester { $name }
arc-op-extract-here = Extraire { $name } ici
arc-op-extract-to = Extraire { $name } vers…
arc-op-delete = Supprimer { $name }
arc-op-add = Ajouter des fichiers à une archive…
job-test = Test de l'archive
msg-dirs-first-on = dossiers d'abord
msg-dirs-first-off = dossiers mêlés aux fichiers
msg-selected-first-on = fichiers sélectionnés d'abord
msg-selected-first-off = fichiers sélectionnés dans l'ordre

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Disque
drive-removable = Amovible
drive-optical = Optique
drive-network = Réseau
drive-memory = Mémoire
drive-home = Perso
drive-connection = Connexion
drive-objects = Stockage objet
drive-netbox = NetBox : sessions enregistrées
drive-processes = Liste des processus
drive-temp = Panneau temporaire
temp-added = { $count } dans le panneau temporaire
proc-none = la liste des processus ne se lit pas ici
drive-programs = Programmes installés
drive-services = Services
svc-none = cette machine n'a pas de gestionnaire de services à interroger
temp-not-a-file = les lignes ici ne sont pas des fichiers : il n'y a rien à supprimer
svc-card = Service
svc-card-id = Nom
svc-card-name = Affiché comme
svc-card-state = État
svc-card-description = Description
svc-card-pid = Processus
svc-state-running = en marche
svc-state-stopped = arrêté
svc-state-failed = en échec
svc-state-other = autre
key-service-start = Démarrer
key-service-restart = Relancer
key-service-stop = Arrêter
svc-ask-stop = Arrêter { $name } ?
svc-ask-restart = Relancer { $name } ?
btn-service-stop = Arrêter
btn-service-restart = Relancer
svc-working = { $name } : je demande au système…
svc-started = { $name } démarré
svc-stopped = { $name } arrêté
svc-restarted = { $name } relancé
svc-needs-root = { $name } : il faut être root — le programme ne le demandera pas à ta place
svc-failed = { $name } : { $error }
prog-none = rien ici ne dit ce qui est installé
prog-card = Programme installé
prog-no-uninstall = rien ne dit comment retirer { $name }
prog-uninstall-ready = la commande qui retire { $name } est dans la ligne de commande, non exécutée
prog-card-name = Nom
prog-card-version = Version
prog-card-publisher = Éditeur
prog-card-size = Place sur le disque
prog-card-installed = Installé
prog-card-location = Dossier
prog-card-uninstall = Retiré par
drive-sockets = Liste des sockets
sock-none = rien n'écoute ici, ou la table n'est pas lisible
sock-no-process = rien ne dit quel processus tient cette socket
proc-card = Processus
proc-gone = { $name } ne tourne plus
proc-card-name = Nom
proc-card-pid = PID
proc-card-parent = Parent
proc-card-user = Utilisateur
proc-card-state = État
proc-card-threads = Fils
proc-card-memory = Mémoire
proc-card-mapped = Réservée
proc-card-bits = Bits
proc-card-started = Démarré
proc-card-exe = Exécutable
proc-card-cwd = Dossier
proc-card-command = Ligne de commande
proc-card-group = Groupe
proc-card-sid = SID
proc-card-priority = Priorité
proc-card-session = Session
proc-card-uptime = En cours depuis
proc-card-cpu-user = Temps processeur, le sien
proc-card-cpu-kernel = Temps processeur, dans le noyau
proc-card-handles = Descripteurs ouverts
proc-card-faults = Défauts de page
proc-card-private = Mémoire privée
proc-card-swap = Sur le swap
proc-card-io-read = Lus
proc-card-io-write = Écrits
proc-card-memory-peak = Pic du jeu de travail
proc-card-private-resident = Jeu de travail privé
proc-card-swap-peak = Pic du fichier d’échange
proc-card-virtual = Octets virtuels
proc-card-virtual-peak = Pic des octets virtuels
proc-card-paged-pool = Pool paginé
proc-card-nonpaged-pool = Pool non paginé
proc-card-io-other = Autres E/S
proc-card-io-read-ops = Opérations de lecture
proc-card-io-write-ops = Opérations d’écriture
proc-card-io-other-ops = Autres opérations
proc-kill-title = Terminer le processus
proc-kill-ask = Terminer { $name } [{ $pid }] ? Ce qu’il n’a pas enregistré est perdu.
proc-killing = Arrêt de { $name }…
proc-killed = { $name } terminé
proc-kill-failed = { $name } ne peut pas être terminé : { $why }
proc-card-env = Environnement
proc-card-modules = Modules chargés
proc-card-version = Version
proc-card-description = Description
drives-hint = ↑↓ choisir · 0-9 panneaux · Ctrl+1..9 colonnes · F9 toutes · Ctrl+R relire
drives-title = Changer de disque : quoi afficher
drives-col-type = le type
drives-col-device = le périphérique
drives-col-label = l'étiquette
drives-col-fs = le système de fichiers
drives-col-size = les tailles
drives-col-removable = les lecteurs amovibles
drives-col-panels = les panneaux qui ne sont pas des disques
drives-col-optical = les lecteurs optiques
drives-col-network = les partages réseau
drives-shown = { $what } affiché
drives-hidden = { $what } masqué
drives-reread = disques relus
bookmarks-hint = Entrée aller · Inser garder · Suppr retirer · F4 nommer · Échap fermer

# The search dialog (F7), which asks the whole question in one place.
search-title = Rechercher
search-for = Rechercher
search-mode = Comme
search-mode-text = texte
search-mode-hex = hex
search-mode-regex = expression régulière
search-case = Respecter la casse
search-words = Mots entiers uniquement
btn-find-next = Suivant
btn-find-prev = Précédent
help-chapter-about = Ce que c'est
help-chapter-panels = Les panneaux
help-chapter-files = Fichiers : copier, déplacer, chercher, archives
help-chapter-viewer = La visionneuse
help-chapter-editor = L'éditeur, le volet hexadécimal et l'assembleur
help-chapter-network = Les serveurs
help-chapter-config = Configuration, touches et couleurs
help-chapter-keys = Les touches, telles qu'elles sont affectées
help-chapter-commands = Toutes les commandes et leurs touches
help-chapter-options = Tous les réglages et leur valeur
help-here-panels = Les panneaux
help-here-viewer = La visionneuse
help-here-editor = L'éditeur
help-here-hex = L'éditeur hexadécimal
help-here-dialog = Cette boîte de dialogue
help-in-english = Ce chapitre n'est pas encore traduit dans votre langue : il suit en anglais. Les touches et les réglages qu'il nomme sont les mêmes dans toutes les langues.
help-about-byline = FinalCommander { $version } — par Matteo Baccan
key-ok = OK
key-cancel = Annuler
key-dialog-move = Déplac
key-tree = Arbre
key-config = Config
browse-local-only = un fichier se choisit sur cette machine ; un chemin sur un serveur se tape, et Tab le complète
key-browse = Parcour
table-prompt-filter = Lignes contenant
table-sorted = triées par { $column }
table-sorted-back = triées par { $column }, à l'envers
table-unsorted = de nouveau dans l'ordre du fichier
table-filtered = { $rows } lignes restantes
table-unfiltered = toutes les lignes de nouveau
table-cut-sorted = des { $rows } premières lignes, tout ce qui a été lu
table-too-many = trop d'enregistrements pour les trier ou les filtrer ici
table-copied =
    { $rows ->
        [one] une ligne dans le presse-papiers
       *[other] { $rows } lignes dans le presse-papiers
    }
table-headed = la première ligne est l'en-tête
table-not-headed = la première ligne est une donnée : les colonnes portent leur numéro
table-named-fields = ce format nomme lui-même ses champs : pas d'en-tête à deviner

# Splitting and combining (SPEC 03 §9)
split-title = Découper le fichier
split-target = Dans
split-size = Chaque morceau
split-size-hint = 1440K, 700M, 4G — le dernier morceau est ce qui reste
split-count = Ou en
split-count-hint = morceaux — un nombre ici l'emporte sur la taille
split-bad-count = { $count } n'est pas un nombre de morceaux : deux ou plus
split-local-file = Découper et réassembler marchent sur un fichier de cette machine
split-bad-size = « { $size } » n'est pas une taille : écrivez 700M ou 4G
combine-not-a-piece = Ce n'est pas un morceau : les morceaux d'un fichier découpé finissent par .001, .002, … ou ont un .crc
pieces-working = Découpage ou réassemblage : { $done } sur { $total }
pieces-cancelled = Arrêté, et ce qui avait été écrit est retiré
split-done = { $parts } morceaux écrits, avec { $list } à côté
combine-done-checked = { $name } réassemblé à partir de { $parts } morceaux, { $bytes }, conforme à son .crc
combine-done-unchecked = { $name } réassemblé à partir de { $parts } morceaux, { $bytes } — pas de .crc pour le vérifier
pieces-failed = Pas fait : { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Lecture de tous les fichiers sous { $path }…
branch-title = Branche : { $path }
branch-done = { $count } fichiers, de tous les dossiers en dessous · Ctrl+Alt+B revient
branch-truncated = { $count } fichiers, et la lecture s'est arrêtée là : la branche est plus grande qu'une liste utile
branch-not-here = La vue en branche est celle d'un dossier, et ce panneau est une liste

## The release check (SPEC 11 §5).
menu-check-update = Rec&hercher les mises à jour
opt-update-check = Chercher une nouvelle version une fois par jour
btn-open-page = Ouvrir la page
update-checking = Demande de la dernière version à GitHub…
update-title = Mises à jour
update-available = FinalCommander { $version } est sorti (ceci est la { $current }).
update-available-status = FinalCommander { $version } est sorti : Options → Rechercher les mises à jour
update-current = C'est la dernière version, la { $version }.
update-failed = Impossible de demander la dernière version : { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Rechercher les fichiers en double
dupes-title = Fichiers en double
dupes-recursive = Chercher dans les sous-dossiers
dupes-min-size = Plus petit fichier
dupes-min-size-hint = Une taille comme l'écrivent les filtres : 1, 100K, 5M. Les fichiers vides ne sont jamais comparés.
dupes-select = Sélectionner toutes les copies sauf la plus ancienne
btn-search = Chercher
dupes-not-here = La recherche de doublons part d'un dossier ou de la sélection.
dupes-bad-size = Ce n'est pas une taille : { $size }
dupes-looking = Recherche des fichiers en double…
dupes-progress = Doublons : passe { $pass } sur 3, { $done }/{ $total }
dupes-none = Aucun doublon parmi { $count } fichiers.
dupes-panel = Doublons : { $groups } groupes
dupes-done = { $groups } groupes de doublons, { $size } dans les copies.
dupes-truncated = Arrêtée tôt : { $groups } groupes pour l'instant, { $size } dans les copies.

## SQLite queries (SPEC 04 §4b).
sql-empty = Écrire une requête après sql:, par exemple sql: SELECT * FROM t
sql-no-database = sql: demande une base SQLite : entrez-y, ou placez le curseur sur une.
sql-running = Requête en cours…
sql-failed = La requête a échoué : { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Aucun service d'IA configuré : ajoutez un bloc [[ai]] à fcmd.toml (name, url, model, api, token_env).
ai-title = Demander à une IA
ai-no-thread = aucune réponse à poursuivre : demandez d'abord avec `ai:`, ou placez le curseur sur une dans le dossier des réponses
ai-nothing-to-explain = rien à expliquer ici : ouvrez un fichier dans la visionneuse ou l'éditeur, ou demandez juste après une commande qui a échoué
ai-explain-question = Qu'est-ce que c'est, dans { $name } ? Explique-le brièvement.
ai-tokens = { $in } jetons en entrée, { $out } en sortie
ai-none-panel = Aucun service d’IA — F7 en ajoute un
ai-service-removed = { $name } retiré
ai-probing = Connexion à { $name } sur { $host }…
ai-probe-ok = { $host } a répondu HTTP { $status } en { $ms } ms, { $models } modèles — tapez une question et Entrée
ai-probe-no-model = (le modèle { $model } n’y est pas)
ai-row-context = contexte
ai-row-conversation = conversation
ai-session-title = IA : { $name } · { $model } — { $files } fichiers en contexte
ai-context-added = { $count ->
        [one] Un fichier
       *[other] { $count } fichiers
    } en contexte
ai-context-nothing = Rien à ajouter : seuls les fichiers de ce disque vont dans le contexte
ai-command-ready = la commande est sur la ligne — lisez-la avant d'appuyer sur Entrée
ai-commit-question = Écris un message de commit pour les modifications indexées.
ai-staged-diff = le diff indexé
ai-send-diff = Le diff indexé part vers { $host }
ai-send-files = { $count } fichiers, { $size }, seront envoyés à { $host } avec la question.
ai-asking = Je demande à { $name }…
ai-busy = { $name } réfléchit… { $secs } s · Échap interrompt
ai-conversation-new = Une nouvelle conversation : la prochaine question la commence.
ai-live-empty = Tapez une question sur la ligne de commande : elle va à { $name }, et la conversation s’affiche ici.
ai-failed = L'IA n'a pas répondu : { $why }
ai-stopped = interrompue : la question à { $name } a été abandonnée, rien n'a été écrit
ai-context-title = Fichiers du contexte
ai-context-all = Tous
ai-context-dropped = { $count ->
        [one] un fichier retiré du contexte
       *[other] { $count } fichiers retirés du contexte
    }
ai-context-hint = Entrée ou Suppr : le retirer du contexte — le fichier reste sur le disque · F3 : ce qui part · Échap
ai-threads-title = Conversations
ai-threads-hint = Entrée : reprendre cette conversation · Échap
ai-thread-resumed = reprise de { $name }
ai-next-estimate = suivante ≈ { $tokens }
ai-next-over = suivante ≈ { $tokens }, plus que le modèle ne contient
ai-send-estimate = Environ { $tokens } tokens en tout, avec la conversation jusqu’ici.
ai-send-estimate-of = Environ { $tokens } tokens en tout, avec la conversation jusqu’ici, sur les { $max } que le modèle contient.
ai-send-over = Environ { $tokens } tokens en tout : plus que les { $max } que le modèle contient, le début pourrait être coupé. F8 retire un fichier.
ai-send-question = La question part vers { $host }.
btn-send = Envoyer

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Ce qui a changé
git-log = Le journal
git-blame = Blame de { $name }
git-branches = Branches
git-local-only = git travaille sur un dossier de ce disque.
git-running = Je demande à git…
git-failed = git : { $why }
git-clean = Rien n'a changé.
git-status-title = Modifiés : { $staged } indexés, { $unstaged } non
git-side-conflict = conflit
git-side-staged = indexé
git-side-changed = non indexé
git-side-untracked = non suivi
git-kind-modified = modifié
git-kind-added = nouveau
git-kind-deleted = supprimé
git-kind-renamed = renommé
git-kind-copied = copié
git-kind-type = type changé
git-kind-conflict = à résoudre
git-as-tree = Afficher les changements en arbre
git-as-list = Afficher les changements en liste
git-version-head = dernier commit
git-version-index = indexé
git-version-disk = sur le disque
git-discard-title = Abandonner les changements
git-discard-body = Abandonner les changements { $count ->
        [one] d’un fichier
       *[other] de { $count } fichiers
    } ?
git-discard-worktree = comme dans l’index :
git-discard-both = comme au dernier commit :
git-discard-added = hors de l’index, gardé sur le disque :
git-discard-trash = à la corbeille :
git-discard-warning = Ce qui revient à l’index ou au dernier commit ne peut pas être annulé.
git-hint = Entrée : choisir · Échap : fermer

## The editor's character list and word completion.
charmap-title = Caractères
charmap-hint = Tapez pour filtrer (un caractère ou son code, U+00E9) · Entrée : insérer · Échap : retour
editor-no-completion = Aucun autre mot du fichier ne commence ainsi.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Copié : { $url }

## The system log (SPEC 06 §16).
menu-syslog = Journal système
syslog-reading = Lecture du journal système…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Retirer les lignes qui correspondent (! devant : ne garder qu'elles)
editor-lines-deleted = Lignes retirées : { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = écraser si la taille diffère

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Si un fichier ne peut être copié
copy-error-ask = Demander
copy-error-skip = L'ignorer et continuer
copy-error-stop = Arrêter la copie

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Dates copiées : Ctrl+Maj+V les met dans la boîte d'un autre fichier
attr-times-none = Rien de copié : d'abord Ctrl+Maj+C dans la boîte d'un fichier
attr-times-pasted = Dates collées : OK les applique

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Remplacements
replace-tpl-hint = Entrée l'exécute · Inser garde un récent sous un nom · Suppr oublie un gardé · Échap
replace-tpl-name = Garder ce remplacement sous le nom
replace-tpl-kept = Gardé sous {$name}
replace-tpl-empty = Aucun remplacement encore : d'abord Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Chaque dossier garde sa vue et son tri

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Limite de vitesse, Ko/s (vide : aucune)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (courrier, lecture seule)
site-protocol-imaps = IMAPS (courrier sur TLS, lecture seule)

# The AI services as a list, like the sessions
ai-services-title = Services d'IA
ai-services-hint = Entrée : celui que ai: interroge · Inser nouveau · F4 modifier · Suppr retirer · Échap
ai-service-title = Service d'IA
ai-field-name = Nom
ai-field-api = Protocole
ai-field-url = Adresse
ai-field-model = Modèle
ai-field-token-env = Clé dans la variable
ai-field-token-note = Vide : la clé est demandée une fois par session et jamais écrite.
ai-field-system = Ce qu'on lui dit d'abord
ai-service-incomplete = Un service veut un nom et une adresse
ai-service-default = ai: interroge maintenant {$name}
ai-key-prompt = La clé de {$name} (gardée jusqu'à la sortie)
ai-api-openai = OpenAI et compatibles
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (sur cette machine)
menu-ai = Services d'IA

# The secrets file
vault-password = Le mot de passe du fichier de secrets de fcmd
vault-new = Garder cette clé dans le fichier chiffré de secrets de fcmd : choisissez son mot de passe (vide : cette session seulement)
vault-refused = Le fichier de secrets ne s'est pas ouvert : {$why}

# Programmes externes
drive-tools = Programmes externes
drive-ai = IA
drive-git = Git
tools-found = trouvé
tools-missing = non installé
tools-summary = { $found } programmes externes trouvés sur { $total } ; F3 dit à quoi sert chacun, F4 installe un programme manquant
tools-card = Programme externe
tools-card-program = Programme
tools-card-where = Emplacement
tools-card-version = Version
tools-card-for = Sert à
tools-card-install = Pour l'installer
tools-card-why = Pourquoi fcmd le lance au lieu de le faire lui-même : F1, « Programs from elsewhere ».
tools-install-any = installez le paquet qui fournit { $name } et vérifiez qu'il est dans le PATH
tools-already = { $name } est déjà là
tools-for-archives = archives : { $what }
tools-for-decompile = Shift+F6 dans la visionneuse sur { $what }
tools-for-git = le panneau Git (Ctrl+Alt+G) et la branche dans l'invite
tools-for-video = les images d'une vidéo dans la visionneuse et l'aperçu rapide (F3, Ctrl+Q)
tools-for-open = ouvrir un lien dans le navigateur
tools-for-syslog = Commandes → Journal système
tools-for-services = le panneau des services
tools-card-site = Site web
tools-no-package = Aucun gestionnaire de paquets de cette machine ne fournit { $name } à notre connaissance. Ouvrir sa page de téléchargement ?
tools-install-ask = Installer { $name } en exécutant, dans un terminal :
tools-install-how = Dans le terminal, sudo peut demander votre mot de passe. Ctrl+Alt+E revient aux panneaux, et Ctrl+R dans ce panneau cherche à nouveau.
tools-install-else = En cas d'échec : { $url }
tools-install-failed = fcmd : { $name } n'a pas été installé. Téléchargez-le depuis { $url }
tools-installing = installation de { $name } dans un terminal : Ctrl+Alt+E revient aux panneaux, Ctrl+R y cherche à nouveau
btn-install = Installer

# Configuration des plugins
plugins-config-title = Plugins
plugins-config-hint = Entrée : activer ou désactiver · F3 ce qu'il est et apporte · Échap
plugins-broken = non chargé — F3 dit pourquoi
plugins-cannot-toggle = un plugin qui ne s'est pas chargé ne peut pas être activé : F3 dit ce qui ne va pas
plugins-on = { $id } est activé : ses commandes sont déjà dans F11
plugins-on-restart = { $id } est activé : ses commandes sont déjà dans F11, le reste au prochain démarrage
plugins-off = { $id } est désactivé : ses commandes ont quitté F11
plugins-off-restart = { $id } est désactivé : ses commandes ont quitté F11, le reste part au prochain démarrage
plugins-state-on = activé
plugins-state-off = désactivé
plugins-card-id = Id
plugins-card-name = Nom
plugins-card-version = Version
plugins-card-author = Auteur
plugins-card-about = Description
plugins-card-dir = Dossier
plugins-card-state = État
plugins-card-brings = Apporte
plugins-card-commands = Commandes (F11) :
plugins-card-notes = Signalé au chargement :
plugins-none = aucun plugin trouvé · ils se trouvent dans { $dir }, un dossier chacun avec un plugin.toml

# The git panel
git-file-log = Le journal de { $name }
git-commit = Valider ce qui est indexé…
git-pull = Pull
git-push = Push
git-pushing = Push en cours…
git-pulling = Pull en cours…
git-commit-prompt = Le message du commit
git-commit-empty = Un commit demande un message.
git-already-on = { $branch } est la branche courante.
git-switch-ask = Passer à { $branch } ? Les fichiers sur lesquels les deux branches diffèrent changent sur le disque.
git-done = git : { $said }
git-branches-title = Branches : { $count }
git-log-title = Journal : { $count }
git-log-of-title = Journal de { $of } : { $count }
btn-switch = Changer
key-git = Git
key-git-diff = Diff
key-git-stage = Indexer
key-git-unstage = Désindex
key-git-discard = Abandonner
key-git-commit = Commit
key-git-show = Voir
key-git-log = Journal
key-git-changes = Modifs
key-git-pull = Pull
key-git-push = Push
key-git-branches = Branches

undo-title = Annuler
btn-undo = Annuler
menu-undo = Annuler la dernière opération
menu-strip-meta = Retirer les métadonnées des images…
undo-nothing = rien à annuler : aucune copie, déplacement, suppression vers la corbeille ni nouveau dossier depuis le lancement
undo-copy =
    { $count ->
        [one] Envoyer à la corbeille le fichier créé par la copie :
       *[other] Envoyer à la corbeille les { $count } fichiers créés par la copie :
    }
undo-move =
    { $count ->
        [one] Remettre le fichier à sa place :
       *[other] Remettre les { $count } fichiers à leur place :
    }
undo-trash =
    { $count ->
        [one] Récupérer le fichier dans la corbeille :
       *[other] Récupérer les { $count } fichiers dans la corbeille :
    }
undo-mkdir = Retirer le nouveau dossier, s'il est encore vide :
strip-title = { $count ->
        [one] Métadonnées d’une image
       *[other] Métadonnées de { $count } images
    }
strip-all = Tout — texte, EXIF, XMP, IPTC, commentaires, C2PA
strip-ai = Seulement ce qu’un modèle d’images a écrit — prompt, workflow, réglages
strip-camera = Tout sauf l’EXIF de l’appareil et le copyright — pas de GPS
strip-hint = Entrée : les retirer ; l’image n’est pas réencodée, Ctrl+Alt+Z la remet comme avant · Échap
strip-nothing = aucun PNG, WebP ou JPEG sur ce disque sélectionné ou sous le curseur
strip-done = { $count ->
        [one] une image
       *[other] { $count } images
    } sans métadonnées ({ $blocks } blocs retirés), { $clean } n’en avaient pas · Ctrl+Alt+Z les remet
undo-restore = { $count ->
        [one] Remettre l’image avec ses métadonnées :
       *[other] Remettre les { $count } images avec leurs métadonnées :
    }
undo-done =
    { $count ->
        [one] un nom de retour à sa place
       *[other] { $count } noms de retour à leur place
    }
slow-drives = la liste des lecteurs a pris { $secs } s
slow-drives-late = la liste des lecteurs a attendu { $secs } s : { $drives } n’ont pas répondu à temps et sont montrés sans détails — un lecteur réseau dont le serveur est absent ?
slow-listing = lire { $what } a pris { $secs } s
# editor search and undo (0.41)
btn-find-all = Toutes les occurrences
btn-replace-all = Tout remplacer
btn-replace-one = Remplacer le suivant
editor-replace-title = Remplacer
editor-replace-style = Conserver le style (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = Dans une expression régulière, $1 et { "${name}" } sont ce qu’un groupe a capturé, $$ un dollar
editor-matches-title = Lignes avec `{ $query }` : { $count }
editor-matches-hint = Tapez pour filtrer · Entrée : aller à la ligne · Échap : retour


# sorting and extended attributes (0.41)
opt-sort-case = Majuscules et minuscules dans le tri
opt-sort-case-auto = comme ce système
opt-sort-case-yes = les distinguer
opt-sort-case-no = un seul alphabet
copy-keep-xattrs = Garder les attributs étendus

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Écrire le fichier en
editor-convert-hint = Tapez pour filtrer · Entrée : convertit (F2 l'écrit, Ctrl+Z l'annule) · Échap : retour
editor-convert-eol = Fin de ligne : { $eol }
editor-convert-bom-on = Marque d'ordre des octets (BOM) : écrite
editor-convert-bom-off = Marque d'ordre des octets (BOM) : non écrite
editor-convert-page = Page de codes : { $page }
editor-converted-eol = { $name } : les lignes finiront par { $what } au prochain enregistrement
editor-converted-bom-on = { $name } : le prochain enregistrement écrit la marque d'ordre des octets
editor-converted-bom-off = { $name } : le prochain enregistrement n'écrit pas la marque d'ordre des octets
editor-converted-page = { $name } : le prochain enregistrement écrit en { $what }
editor-convert-same = Le fichier est déjà écrit ainsi
editor-convert-no-bom = { $page } n'a pas de marque d'ordre des octets : seuls UTF-8 et UTF-16 en ont une
editor-convert-lossy = { $page } ne peut pas contenir tous les caractères de { $name } : rien n'a été changé
btn-reload = Recharger
btn-what-changed = Différences
editor-diff-buffer = dans l'éditeur
editor-diff-hint = Cette comparaison montre seulement ce qui a changé — Échap revient à la question


# job journal and saved syncs (0.41)
journal-left = Copies laissées inachevées par la dernière session : { $count } · Alt+Shift+F6 les liste
journal-title = Travaux inachevés
journal-hint = Entrée reprendre · Suppr abandonner · Échap fermer
journal-none = Aucun travail n'est resté inachevé
journal-row = { $kind } : { $what } → { $target } · fichiers faits : { $done }
journal-row-unreadable = illisible : { $file } ({ $error })
journal-unreadable = { $file } : ce n'est pas un journal de travail que cette version sait lire ({ $error }). Alt+Shift+F6 le liste, et Suppr l'y retire
journal-resumed = Reprise de { $kind } : { $done } fichiers étaient déjà faits
journal-discarded = Le travail inachevé a été abandonné, avec ce qu'il avait écrit à moitié
journal-cannot-resume = Ce journal est illisible ; Suppr le retire
menu-jobs-left = Travaux inachevés
sync-report = Écrire un rapport de l'exécution dans (un fichier)
sync-name = Enregistrer cette paire sous (un nom)
sync-saved-button = Enregistrées…
sync-saved-title = Synchronisations enregistrées
sync-saved-hint = Entrée lancer · F4 modifier · Suppr oublier · Échap fermer
sync-saved-none = Aucune synchronisation n'est encore enregistrée : donnez-lui un nom dans le dialogue (Ctrl+Shift+Y)
sync-saved-removed = { $name } n'est plus enregistrée
sync-report-local = Le rapport doit être un fichier de cette machine
sync-report-written = Le rapport de la synchronisation est dans { $path }
sync-report-not-written = Le rapport n'a pas pu être écrit dans { $path } : { $error }
sync-report-title = Rapport de synchronisation
sync-report-name = Nom : { $name }
sync-report-left = Gauche : { $path }
sync-report-right = Droite : { $path }
sync-report-to-right = Copiés de gauche → à droite : { $count }
sync-report-to-left = Copiés de droite → à gauche : { $count }
sync-report-cut-right = Retirés à droite : { $count }
sync-report-cut-left = Retirés à gauche : { $count }
sync-report-skipped = Laissés tels quels bien que différents : { $count }
sync-report-failed = Échoués : { $count }
sync-report-stopped = Arrêté en cours de route, ceux-ci ont donc pu être faits ou non : { $count }
sync-report-not-done = Non faits, parce qu'une étape précédente a échoué ou a été arrêtée : { $count }
menu-sync-saved = Synchronisations enregistrées

# hex editor search
hex-search-wrapped = `{ $query }` : trouvé en repartant de l'autre bout du fichier
hex-replace-title = Remplacer
hex-replace-with = Remplacer par
btn-replace = Remplacer
hex-replace-ask = Remplacer { $was } en { $at } par { $with } ?
hex-replace-length = le remplacement fait { $with } octets et la correspondance { $was } : cela décalerait tout ce qui suit — appuyez d'abord sur Inser pour passer en mode insertion

# pixels and video (0.41)
opt-viewer-pixels = Images
opt-viewer-pixels-auto = vrais pixels là où le terminal les dessine
opt-viewer-pixels-kitty = protocole graphique de kitty
opt-viewer-pixels-iterm = images en ligne d'iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = toujours des demi-blocs
video-needs-program = ceci est une vidéo ({ $what }) : ses images sont dessinées par `{ $program }`, qui n'est pas installé sur cette machine — F11 lit toujours le conteneur
video-no-frame = aucune image de cette vidéo : { $why }
video-playing = lecture à { $fps } images par seconde, sans le son — Espace met en pause
video-paused = en pause à { $at } — Espace reprend, PgPréc et PgSuiv avancent d'un dixième
video-ended = la vidéo est terminée — Espace la relance


# finding things
menu-saved-searches = Recherches enregistrées
drive-searches = Recherches enregistrées
find-keys = F2 garde cette recherche sous un nom · F9 celles gardées
find-save-name = Garder cette recherche sous le nom
find-saved = recherche gardée sous { $name }
find-saved-none = aucune recherche gardée : F2 dans la fenêtre de recherche en garde une
find-saved-title = Recherches enregistrées — Entrée ouvre, F4 modifie, Suppr oublie
find-saved-running = recherche de { $name }…
find-saved-gone = recherche { $name } oubliée
find-loaded = recherche { $name } chargée
find-fuzzy = Nom approché : les lettres ci-dessus, dans l’ordre, le meilleur d’abord
find-meta = Métadonnées (artist=bowie; camera=)
find-index = Interroger l’index du système au lieu de parcourir (peut manquer les fichiers récents)
tools-for-index = l’index qu’Alt+F7 peut interroger au lieu de parcourir
menu-frecent = Dossiers fréquents
frecent-none = aucun dossier visité pour l’instant
frecent-gone = { $path } retiré des dossiers fréquents
frecent-hint = tapez des lettres du chemin · Entrée y aller · Suppr oublier · Échap fermer

# ai third round
ai-redact-off = Le masquage est désactivé : tout part tel quel
ai-redact-none = Aucun secret à masquer
ai-redact-count = Secrets masqués avant l'envoi : { $count } ({ $kinds })
ai-redacted = secrets masqués : { $count }
ai-preview-title = Ce qui part vers { $name } ({ $host })
ai-preview-instruction = Instruction
ai-preview-earlier = La conversation jusqu'ici
ai-preview-question = La question et ses fichiers
ai-preview-key = F3 montre ce qui part
ai-preview-reading = Lecture de ce qui partirait…
key-ai-redact = Masque
ai-redact-now-on = Masquage activé : clés, jetons, clés privées et mots de passe sont masqués avant tout envoi
ai-redact-now-off = Masquage désactivé : ce qui part, part tel quel
ai-panel-redact-on = secrets masqués
ai-panel-redact-off = secrets non masqués
key-ai-local = Local
ai-row-refused = pas cette machine : refusé
ai-panel-local-only = local uniquement
ai-local-now-on = Local uniquement activé : un service qui n'est pas sur cette machine est refusé, et rien ne lui est envoyé
ai-local-now-off = Local uniquement désactivé : tous les services de la liste peuvent être interrogés
ai-local-refused = Local uniquement est activé : { $name } répond sur { $host }, qui n'est pas cette machine — rien n'a été envoyé (F6 dans le panneau AI le désactive)
ai-rename-prompt = Que doivent devenir les { $count } noms ? (en mots)
ai-send-names = L'instruction et { $count } des { $total } noms — les noms seuls, rien des fichiers — partent vers { $host }.
ai-rename-ready = le modèle proposé est dans le dialogue : lisez l'aperçu, corrigez ce qui ne va pas, seul OK renomme
ai-rename-waiting = le modèle de renommage proposé est prêt : Ctrl+Shift+F6 ouvre le dialogue avec lui
ai-rename-unreadable = le modèle n'a pas répondu par un modèle de renommage : sa réponse est dans le dossier des réponses (ai:), rien n'a été renommé
ai-rewrite-prompt = Que doit devenir le bloc marqué ? (en mots)
ai-rewrite-no-block = marquez d'abord un bloc (Maj+flèches) : la réécriture remplace exactement ce qui est marqué, et un bloc en colonne n'a pas de forme où revenir
ai-rewrite-read-only = ce fichier est en lecture seule ici : une réécriture n'aurait nulle part où aller
ai-rewrite-before = le bloc
ai-rewrite-after = tel que le modèle l'a réécrit
ai-rewrite-hint = La réécriture du modèle face au bloc — Entrée la met à la place du bloc, Échap laisse le bloc tel quel
diff-hint-rewrite = Entrée remplace le bloc · Échap le laisse
ai-rewrite-applied = le bloc a été remplacé par la réécriture du modèle — un seul Ctrl+Z le remet
ai-rewrite-left = le bloc est resté tel quel
ai-rewrite-stale = le bloc n'est plus ce qu'il était à la demande : rien n'a été remplacé — la version du modèle est dans le dossier des réponses (ai:)
ai-rewrite-same = le modèle a rendu le bloc tel quel : rien à remplacer
ai-answer-kept = la réponse est { $name } dans le dossier des réponses (ai:) — l'éditeur est resté devant
ai-failure-question = Cette commande s'est terminée avec le code de sortie { $code } : `{ $command }` — pourquoi a-t-elle échoué, et qu'est-ce qui la corrigerait ? Brièvement.
ai-send-failure = La commande qui a échoué et les { $lines } dernières lignes qu'elle a affichées partent vers { $host }.

# archives and jobs
arc-test-whole = { $name } : { $files } entrées vérifiées, { $bytes } — toutes intactes
arc-test-bad-title = { $name } : { $bad } entrées sur { $files } sont endommagées

# extending it
plugin-column-failed = { $column } : { $error } — la colonne reste vide pour les fichiers sans réponse
plugins-card-columns = Colonnes (comme les nomme un [[column_set]]) :
usermenu-far-row = Ce qui a été lu autrement que dans FarMenu.ini ({ $count })
usermenu-far-notes = le menu vient du FarMenu.ini de FAR : { $count } choses ont été lues autrement — la dernière ligne les énumère
usermenu-far-not-run = Rien n'a été exécuté.
find-age = Modifié, il y a (jours : 7, >=30, 7-30)
find-bad-age = `{ $text }` n'est pas un âge en jours — { $error }
find-label-age = { $days } j
find-keys-ai = Ctrl+Alt+Q dit la recherche en mots : un modèle de langage remplit ces lignes
ai-find-prompt = Que cherchez-vous ? (en mots)
ai-send-words = Ces mots — et rien des panneaux : aucun nom, aucun chemin, aucun fichier — vont à { $host }.
ai-find-ready = la recherche du modèle est dans le dialogue : lisez les lignes, corrigez ce qui ne va pas, et seul OK cherche
ai-find-waiting = la recherche du modèle est prête : Alt+F7 ouvre le dialogue avec elle
ai-find-unreadable = le modèle n'a pas répondu par une recherche : sa réponse est dans le dossier des réponses (ai:), et rien n'a été cherché
ai-find-left-out = { $said } — PAS dans les lignes, qui ne savent pas le dire : { $what }
# treemap and compare
usage-map-title = Occupation du disque, tout l'arbre : { $path }
usage-map-walking = Mesure… fichiers : { $files }, { $size } jusqu'ici
usage-map-where = { $path } · { $size } · fichiers : { $files }
usage-map-unreadable = dossiers non lus : { $count }
usage-map-rest = fichiers plus petits : { $count }
usage-map-others = autres : { $count }
usage-map-empty = Rien dans ce dossier n'occupe de place
usage-map-chosen = { $name } · { $size } · { $share } %
usage-map-hint = ←↑↓→ · Entrée entrer · BkSp remonter · Tab liste · G panneau ici · Échap
usage-map-hint-list = ↑↓ · Entrée entrer · BkSp remonter · Tab carte · G panneau ici · Échap
usage-map-hint-walking = Échap arrête
usage-map-top = C'est le dossier mesuré : il n'y a rien au-dessus ici
usage-map-no-file = Cette ligne représente beaucoup de petits fichiers, pas un seul où mener le panneau
bin-comparing = Comparaison des deux fichiers octet par octet… Échap arrête
bin-bytes = { $count } octets
bin-same = Les deux fichiers sont identiques, octet pour octet : { $bytes } octets chacun
bin-where = différence { $at } sur { $count } · la première à l'offset { $offset } · octets différents : { $differing }
bin-hint = Alt+↑↓ différences · ↑↓ PgUp PgDn Home End lignes · Échap fermer
