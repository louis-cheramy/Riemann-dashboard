# Riemann Dashboard (Rust)

Application desktop native pour explorer la repartition des nombres premiers et visualiser les zeros de la fonction zeta de Riemann (hypothese de Riemann).

## Fonctionnalites

- **Generation** de tous les nombres premiers jusqu'a une borne (crible segmente, format `PRIMEV2` uint64)
- **Histogramme** de repartition sur un intervalle
- **Espacements** entre nombres premiers consecutifs
- **Zeros de Riemann** en 2D (Plot interactif) et 3D (rotation souris + zoom)
- **Affichage des entiers** avec les premiers en rouge
- Lecture memoire mappee du fichier `.bin` (supporte des centaines de millions de premiers)

## Prerequis (Windows)

1. **Rust** : https://rustup.rs/
   ```powershell
   winget install Rustlang.Rustup
   ```
2. **Visual Studio Build Tools** (linker MSVC requis pour compiler sur Windows) :
   ```powershell
   winget install Microsoft.VisualStudio.2022.BuildTools
   ```
   Cochez **« Developpement Desktop en C++ »** ou **VC++ Build Tools** lors de l'installation.

Redemarrez le terminal apres l'installation.

## Compilation

```powershell
cd Riemann-dashboard
cargo build --release
```

L'executable se trouve ici :

```
target\release\riemann-dashboard.exe
```

## Utilisation

### Interface graphique (par defaut)

```powershell
.\target\release\riemann-dashboard.exe
```

ou :

```powershell
cargo run --release
```

Placez `nombres_premiers.bin` dans le meme dossier que l'exe, ou generez-le depuis le panneau lateral de l'application.

### Ligne de commande — generation des premiers

```powershell
.\target\release\riemann-dashboard.exe generate 10000000000
```

Options :

```powershell
riemann-dashboard generate --help
```

## Distribution / installation

Copiez ces fichiers sur la machine cible :

- `riemann-dashboard.exe` (depuis `target\release\`)
- `nombres_premiers.bin` (optionnel, ~3,6 Go pour 10 milliards)

Aucune installation de Python n'est requise. L'application est autonome.

Pour un installateur Windows (`.msi`), vous pouvez utiliser [cargo-wix](https://github.com/volks73/cargo-wix) ou [NSIS](https://nsis.sourceforge.io/).

## Format du fichier binaire

| Element | Detail |
|---------|--------|
| En-tete | `PRIMEV2\x00` (8 octets) |
| Donnees | entiers `uint64` little-endian (8 octets chacun) |
| Compatibilite | ancien format uint32 (sans en-tete) toujours lisible |

## Structure du projet

```
Riemann-dashboard/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI + lancement GUI
│   ├── lib.rs
│   ├── primes/           # Crible + lecture mmap
│   ├── riemann/          # Zeros triviaux / non triviaux
│   └── app/              # Interface egui
├── python/               # Ancienne version Python (legacy)
└── nombres_premiers.bin  # Donnees (non versionne)
```

## Version Python (legacy)

L'ancienne version Streamlit est conservee dans `python/` :

```powershell
pip install streamlit matplotlib numpy plotly
python -m streamlit run python/analyse.py
```

## Depannage

| Probleme | Solution |
|----------|----------|
| `link.exe not found` | Installer Visual Studio Build Tools (C++) |
| `rustc` introuvable | Installer Rust via rustup, redemarrer le terminal |
| Fichier `.bin` introuvable | `riemann-dashboard generate 1000000` ou generer depuis l'GUI |
| Application lente au demarrage | Normal avec un fichier de 3+ Go ; le mmap evite de tout charger en RAM |
