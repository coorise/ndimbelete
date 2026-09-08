# Ndimbelente

Application desktop hors ligne pour la gestion des cotisations de l’**Association Sénégalaise d’Entraide NDIMBELENTÉ**.

Stack : **Tauri 2** + **Leptos (CSR)** + **SQLite** + **Tailwind CSS**.

## Prérequis

- [Rust](https://www.rust-lang.org/) (stable)
- [Bun](https://bun.sh/) (scripts CSS)
- Cibles / outils :

```bash
rustup target add wasm32-unknown-unknown
cargo install tauri-cli --version "^2.0.0" --locked
cargo install trunk --locked
```

Sous Windows, suivez aussi les [prérequis Tauri](https://v2.tauri.app/start/prerequisites/).

## Démarrage

```bash
bun install
bun run css:build   # ou css:watch
cargo tauri dev
```

Build production :

```bash
cargo tauri build
```

## Première connexion

Au premier lancement, l’écran **Configuration initiale** propose :
1. **Créer l’administrateur** (Commissaire aux comptes), ou
2. **Restaurer une sauvegarde** (fichier `.bak` / `.db`) puis redémarrage automatique.

Ensuite, connectez-vous avec l’identifiant créé (ou celui de la sauvegarde).

Base SQLite locale : `~/ndimbelente/data/ndimbelente.db`

Langues : français, anglais, espagnol, allemand (sélecteur dans la barre du haut).
Icône / logo : poignée de mains (entraide) dérivée du matériel de l’association.

## Structure (style Next.js)

```
src/                          # Frontend Leptos (UI)
  app/
    pages/                    # landing, login, dashboard/*
    components/ui|layout|…    # composants réutilisables
    lib/                      # api (invoke Tauri), types, thème
    hooks/                    # contexte auth
src-tauri/src/                # Backend Rust
  db/ commands/ models/ services/
.samples/                     # Excel & images de référence (ignorés par git)
```

## Fonctionnalités

- Accueil + authentification staff
- Dashboard : Overview, Staff/Rôles, Membres, Cotisations, Paramètres
- Cotisations bi-mensuelles (10 €/mois par défaut), dette / paiement / surplus
- Import / export Excel (aperçu de feuille, barre de progression, rapport d’erreurs)
- Reçu imprimable A4 ou POS-58
- Thème clair/sombre, taille de police, couleurs d’organisation

## Cotisations (règles métier)

- Périodes : janv., mars, mai, juil., sept., nov.
- Colonne « dû » / « payé » par période ; absence de paiement = non cotisé
- Surpaiement reporté sur les périodes suivantes
- ≥ 6 mois d’impayés → statut *démissionnaire*
