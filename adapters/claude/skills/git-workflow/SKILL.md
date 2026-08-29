---
name: git-workflow
description: Conventions de branche, de commit et de pull request pour les dépôts suivis par Ariane. À charger dès qu'une session touche à git — créer une branche, écrire un commit, ouvrir une PR.
---

# Workflow git

## Branches

- Une branche par intention, nommée `type/sujet-en-kebab` : `site/ecran-titre`,
  `reflexion/navigation`, `fix/dates-partielles`.
- Elle part de la branche par défaut du dépôt à jour, jamais d'une autre branche
  de travail — sauf empilement assumé, et alors on le dit dans la PR.
- **Jamais de push sur la branche par défaut.** Un push sur `main` est une
  publication.
- **Jamais de `--force`**, jamais de réécriture d'une branche déjà poussée.

## Commits

- Message en français, `type(scope): verbe à l'infinitif ou à la 3ᵉ personne`.
- Le corps dit **pourquoi**, pas quoi : le diff dit déjà quoi. Un commit qui n'a
  rien à expliquer n'a pas de corps.
- Un commit = un pas cohérent. Ni « wip », ni fourre-tout.

## Pull requests

- **Ouvrir la PR directement**, sans demander confirmation, dès que le travail
  tient debout.
- **Titre** : une phrase claire en français. Pas le nom de branche recopié par
  l'outil — `Site/ecran titre` n'est pas un titre.
- **Description : très courte.** Ce que ça change, et pourquoi. Quelques lignes.
  Pas de journal de bord, pas de recopie du diff, pas de tableau de mesures — ça
  vit dans les commits, les ADR et les réflexions du nœud.
- Signaler en une ligne ce qui n'a **pas** été vérifié, quand c'est le cas.

## Où va le raisonnement

Nulle part dans le code. Une décision coûteuse à défaire va dans une **ADR** du
dépôt ; une réflexion en cours va dans `reflexions/` du nœud Ariane. Le code, lui,
se passe de commentaire — voir les règles permanentes de l'utilisateur.
