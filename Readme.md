# Riemann Dashboard

Dashboard interactif pour explorer la répartition des nombres premiers et visualiser les zéros de la fonction zêta de Riemann.

## Vue d'ensemble

Le projet se compose de deux scripts Python :

| Fichier | Rôle |
|---------|------|
| `premier.py` | Génère tous les nombres premiers jusqu'à une borne choisie et les enregistre dans `nombres_premiers.bin` |
| `analyse.py` | Application Streamlit qui lit ce fichier binaire et affiche des graphiques interactifs |

Les nombres premiers sont stockés en binaire (8 octets par entier, format `uint64` little-endian, en-tête `PRIMEV2`). Cela permet de dépasser la limite des 4,3 milliards imposée par l'ancien format 32 bits. Le fichier `nombres_premiers.bin` n'est pas versionné (voir `.gitignore`) : il doit être généré localement avant de lancer le dashboard.

> **Note :** si vous aviez un fichier généré avec l'ancienne version, supprimez-le et relancez `python premier.py`.

## Prérequis

- Python 3.8 ou supérieur
- Les dépendances suivantes :

```bash
pip install streamlit matplotlib numpy plotly
```

## Installation

1. Cloner ou télécharger le dépôt.
2. Installer les dépendances (voir ci-dessus).
3. Générer le fichier de données (étape obligatoire la première fois).

## Étape 1 — Générer les nombres premiers

Lancer le script de crible segmenté :

```bash
python premier.py
```

Le programme demande une borne maximale (ex. `10000000000` pour 10 milliards). Le calcul peut prendre longtemps selon la borne et la machine (plusieurs heures pour 10¹⁰). À la fin, un fichier `nombres_premiers.bin` est créé à la racine du projet (~3,6 Go pour 10 milliards).

## Étape 2 — Lancer le dashboard

```bash
python -m streamlit run analyse.py
```

Streamlit ouvre automatiquement le dashboard dans le navigateur (par défaut sur `http://localhost:8501`).

## Fonctionnalités du dashboard

Une fois lancé, `analyse.py` propose :

- **Histogramme (répartition)** — distribution des nombres premiers sur un intervalle choisi
- **Espacement entre premiers** — histogramme des écarts entre nombres premiers consécutifs
- **Zéros de la fonction zêta de Riemann** — visualisation interactive 2D et 3D (Plotly) des zéros triviaux et non triviaux, avec animation optionnelle
- **Affichage des entiers et des nombres premiers** — liste des entiers d'un intervalle, avec les premiers en rouge

Des champs permettent de définir l'intervalle d'analyse (borne min / max) et d'ajuster les paramètres des graphiques.

## Structure du projet

```
Riemann-dashboard/
├── premier.py              # Génération du fichier binaire de premiers
├── analyse.py              # Dashboard Streamlit
├── riemann_viz.py          # Visualisations 2D/3D des zéros de Riemann
├── nombres_premiers.bin    # Données générées (non versionné)
└── Readme.md
```

## Dépannage

| Problème | Solution |
|----------|----------|
| `Fichier 'nombres_premiers.bin' introuvable` | Exécuter d'abord `python premier.py` |
| `struct.error: 'I' format requires...` ou fichier incomplet | Supprimer `nombres_premiers.bin` et relancer `python premier.py` (ancien format 32 bits) |
| `streamlit` introuvable | Installer avec `pip install streamlit` ou utiliser `python -m streamlit run analyse.py` |
| Intervalle trop petit (espacements) | Choisir un intervalle contenant au moins deux nombres premiers |
