---
name: ariane-session
description: Met à jour le registre des fils de travail (sessions/) d'un nœud Ariane depuis la conversation en cours — range ce qui a été tranché, ouvre un fil si le sujet n'en a pas, signale ceux dont la condition d'archivage est remplie. À invoquer en fin de session, ou quand une conversation a manifestement dérivé sur plusieurs sujets.
---

# ariane-session

Le dossier `sessions/` d'un nœud répond à une question qu'aucun autre n'adresse :
**« dans quelle conversation est-ce que j'ouvre ça ? »**

Un fil est une **lignée de conversations**, pas un sujet. Il a une frontière, un
état, une condition d'archivage. Ce skill l'entretient.

## Ce que ce dossier n'est pas

Ne recopie **rien** ici. Le nœud a déjà trois endroits, et ils gardent leur rôle :

| | Répond à |
|---|---|
| `reflexions/` | *qu'est-ce qu'on décide sur ce sujet ?* |
| `tasks/`, `stories/` | *qu'est-ce qu'on fait ?* |
| `HANDOFF.md` | *où en est-on à l'instant ?* |

Une fiche de fil **pointe** vers eux et dit ce qui reste ouvert. Si tu te
surprends à expliquer un raisonnement dans `sessions/`, il va dans `reflexions/`.

## La procédure

### 1. Situer

Lis `sessions/INDEX.md` du nœud concerné. S'il n'existe pas, va au point 5.

Détermine à quel fil appartient ce qui vient d'être fait. **Une conversation peut
en toucher plusieurs** — c'est même le cas normal, et c'est précisément ce qu'il
faut consigner séparément plutôt que d'écrire un résumé unique.

### 2. Ranger ce qui a été tranché

Pour chaque fil touché, dans sa fiche :

- déplace en **« Ce qui est tranché »** ce qui l'a été, en une ligne, avec son
  motif. Pas le raisonnement — le motif ;
- retire de **« Ce qui reste ouvert »** ce qui ne l'est plus ;
- ajoute ce qui s'est ouvert pendant la session ;
- mets l'état à jour dans l'en-tête **et** dans le tableau de `INDEX.md`.

**Une décision qui a coûté cher à prendre mérite une ADR ou une réflexion, pas une
ligne de fiche.** Écris-la là, et fais pointer la fiche vers elle.

### 3. Vérifier les conditions d'archivage

Chaque fiche porte une condition en dernière section. Elle doit être
**vérifiable** : « les six écrans existent », pas « le design est fini ».

Pour chaque fil, y compris ceux que la session n'a pas touchés : la condition
est-elle remplie ? Si oui, **ne l'archive pas toi-même** — signale-le au
propriétaire avec ce qui le prouve. L'archivage est une décision.

Quand il est décidé : la fiche part dans `archives/sessions/` du nœud, la ligne
de l'`INDEX.md` la suit avec sa date. **Rien n'est supprimé.**

### 4. Repérer un fil qui aurait dû exister

Si la session a traité un sujet qui n'appartient à aucun fil, c'est le signal
d'un fil manquant. **Le critère est unique :**

> Un fil mérite d'exister quand **il peut avancer sans les autres**.

Il se vérifie. Si ouvrir un sujet oblige à rouvrir un autre fil, les deux n'en
font qu'un — n'en crée pas un second. Et une dépendance à sens unique n'est pas
une fusion : un écran a besoin d'un modèle, un modèle n'a jamais besoin d'écran.

Propose le fil, ne le crée pas sans accord.

### 5. Créer le dossier, s'il n'existe pas

`sessions/INDEX.md` avec le tableau des fils et la règle de séparation, puis une
fiche par fil. Le gabarit d'une fiche :

```markdown
---
node: <chemin/du/noeud>
---

# <Nom du fil>

> <état + date>. <une phrase qui dit pourquoi il est dans cet état>

## Ce qu'on y traite
## Où vit le travail
## Ce qui est tranché
## Ce qui reste ouvert
## Condition d'archivage
```

## Règles

- **Ne rien inventer.** Ce qui n'a pas été tranché reste dans « ouvert ». Une
  session qui n'a rien décidé ne produit aucune ligne de « tranché ».
- **Écrire les blocages, pas seulement les avancées.** Un fil bloqué sur une
  action du propriétaire doit le dire, et dire laquelle.
- **Signaler ce qui n'a pas été vérifié.** Une décision prise sur un raisonnement
  et non sur une mesure se marque comme telle.
- **Une fiche courte.** Cinquante lignes. Au-delà, c'est qu'elle explique au lieu
  de pointer.
- Français. Les identifiants et les chemins restent tels qu'ils sont.
