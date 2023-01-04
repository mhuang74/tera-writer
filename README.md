# tera-writer
Command line utility for generating content files via Tera template and OpenAI Completion API.

## OpenAI Access

Create an account on OpenAI, generate an API Key, and set it on *OPENAI_API_KEY* env var.

```
export OPENAI_API_KEY=<your api key>
```

## Help

```
Command line utility for generating content files via Tera template and OpenAI Completion API

Usage: teraw [OPTIONS] <CONTEXT>

Arguments:
  <CONTEXT>  JSON file with topics and prompts for Completion API, or used as data source when Tera template is specified

Options:
  -t, --template <TEMPLATE>            Tera template to inject JSON data into
  -o, --output-path <OUTPUT_PATH>      Output path [Default: current directory]
  -d, --directory-key <DIRECTORY_KEY>  create output directories based on value of this context key
  -h, --help                           Print help information
  -V, --version                        Print version information
  
```

## Example Steps for generating contents for a Zola-powered Blog

1) Use context file with prompts to generate English contents
```
$ RUST_LOG=teraw=info teraw samples/context/2022_top_10_wines_with_prompts.json
```

2) Manually review/edit contents & save it
```
$ subl samples/context/2022_top_10_wines_with_prompts.content.json
```

3) Generate content files with Zola directory convention
```
$ RUST_LOG=teraw=info teraw samples/context/2022_top_10_wines_with_prompts.content.json -t samples/template/index.md --directory-key wine_name -o tmp
```

## Example JSON context file with Prompt Templates

```
{
	"contexts" : [
        {
            "wine_name": "Schrader Cellars Double Diamond Oakville Cabernet Sauvignon from Napa Valley"
        },
        {
            "wine_name": "Fattoria dei Barbi Brunello di Montalcino Riserva DOCG from Tuscany"
        },
        {
            "wine_name": "HDV Hyde de Villaine Hyde Vineyard Chardonnay from Carneros"
        },
        {
            "wine_name": "Chateau Talbot from Saint-Julien"
        },
        {
            "wine_name": "Marchesi Antinori Tignanello Toscana IGT from Tuscany"
        },
        {
            "wine_name": "Robert Mondavi Winery The Estates Cabernet Sauvignon from Oakville"
        },
        {
            "wine_name": "Chateau de Beaucastel Chateauneuf-du-Pape from Rhone"
        },
        {
            "wine_name": "Fattoria Le Pupille 'Saffredi' Maremma Toscana from Tuscany"
        },
        {
            "wine_name": "Quilceda Creek Cabernet Sauvignon from Columbia Valley"
        },
        {
            "wine_name": "Louis Roederer Cristal Millesime Brut from Champagne"
        }
	],
    "prompt_templates" : {
        "wine_description": {
            "tokens": 50,
            "prompt": "Write a one sentence description about: {{wine_name}}"
        },
        "wine_popularity": {
            "tokens": 300,
            "prompt": "Write about what makes this wine so popular: {{wine_name}}"
        },
        "wine_history": {
            "tokens": 300,
            "prompt": "Write about the history of this wine: {{wine_name}}"
        },
        "wine_pairing": {
            "tokens": 300,
            "prompt": "Write about foods that go well with: {{wine_name}}"
        }
    }
}
```

### Example Tera Template with Prompts

```
---
title: {{ wine_name }}
description: {{ openai_completion(tokens=50, prompt="Write a one sentence description about: " ~ wine_name) }}
date: 2022-01-01
extra:
  image: image.webp
taxonomies:
  tags:
---

##### What makes {{ wine_name }} so popular

{{ openai_completion(tokens=300, prompt="Write about what makes this wine so popular: " ~ wine_name) }}

##### History of {{ wine_name }}

{{ openai_completion(tokens=300, prompt="Write about the history of this wine: " ~ wine_name) }}

##### Pairing {{ wine_name }}

{{ openai_completion(tokens=300, prompt="Write about foods that go well with: " ~ wine_name) }}
```
