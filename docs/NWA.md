# Backend NumWorks `.nwa`

## Pipeline

```text
main.klc -> validation Kalcite -> projet Rust no_std -> ELF ARM relocalisable -> .nwa
```

La cible est `thumbv7em-none-eabihf`. Les options de lien sont `--relocatable` et `-no-gc-sections`, conformément au modèle Rust officiel NumWorks. Le binaire contient les sections EADK suivantes :

- `.rodata.eadk_app_name`
- `.rodata.eadk_api_level`
- `.rodata.eadk_app_icon`

Le backend initial produit le Pong natif de démonstration. Il n’embarque ni VM, ni GC, ni interpréteur. Le futur backend MIR remplacera progressivement le générateur spécialisé sans modifier le format de sortie.

## Commande

```bash
kalcite build-nwa examples/pong/src/main.klc --name Pong -o Pong.nwa
```

`--no-build` conserve seulement le projet Rust généré.

## Installation

Une fois le fichier produit, il peut être envoyé avec la page Apps NumWorks ou avec :

```bash
npx --yes nwlink@0.0.16 install-nwa Pong.nwa
```


For firmware-sensitive NumWorks features (manual SVCs, storage, Home/OnOff), see [`NUMWORKS_ADVANCED.md`](NUMWORKS_ADVANCED.md).
