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