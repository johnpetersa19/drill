# Drill (Broca v2)

Ferramenta de análise recursiva de arquivos binários por árvore de nós.

Aceita qualquer `.bin`, `.rom`, `.raw`, `.img` ou `.dat` — firmware, executável,
dump de memória, imagem de disco, ROM de console — e percorre o conteúdo real
usando detectores plugáveis, análise de entropia e validação estrutural,
produzindo um mapa JSON navegável (`esqueleto/manifesto.json`) sem jamais
modificar o arquivo original.

## Arquitetura

```
src/
├── engine/       # motor recursivo: Node, Tree, Entropy, Manifest
├── detectors/    # plugins de detecção (containers, compressão, fs, executáveis...)
├── packers/      # packers simétricos para Broca Reversa
├── output/       # geração de esqueleto/, nos/, relatórios
└── window.rs     # interface GTK4/libadwaita
```

## Saída

```
BROCA_ANALISE/
├── original/          # arquivo intacto + SHA256
├── esqueleto/         # manifesto.json + relatórios de texto
├── nos/               # assembly, pseudo-código, listagens de fs
├── edicoes/           # edições pendentes / aplicadas (Broca Reversa)
└── saida/             # arquivo reconstruído + validação round-trip
```

## Licença

GPL-3.0-or-later
