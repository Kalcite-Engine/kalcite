# Organisation des dépôts

L'organisation GitHub [Kalcite Engine](https://github.com/Kalcite-Engine)
regroupe les projets publics de Kalcite.

| Dépôt | Rôle |
| --- | --- |
| `kalcite` | Workspace Rust principal : langage, runtime, moteur, backends, CLI, éditeur et intégrations. |
| `kalcite-docs` | Site de documentation destiné aux utilisateurs et contributeurs. |
| `kalcite-website` | Site vitrine du projet. |

## Workspace principal

Les crates sous `crates/` restent dans le même dépôt. Elles évoluent ensemble
et sont vérifiées par une seule CI, ce qui évite les contraintes de versions et
de publication entre composants étroitement liés.

## Sous-modules de sites

Le dépôt `kalcite` référence les sources des deux sites comme sous-modules.
Pour récupérer un clone complet :

```bash
git clone --recurse-submodules https://github.com/Kalcite-Engine/kalcite.git
```

Après un clone existant :

```bash
git submodule update --init --recursive
```

Les changements d'un site se font dans son dépôt dédié. Le dépôt `kalcite`
met ensuite à jour le commit référencé afin de produire un état intégré et
reproductible.
