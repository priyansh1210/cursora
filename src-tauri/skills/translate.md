---
name: translate
description: Translate the selection
args: <language>
output: replace
---
Translate the captured text. Keep its formatting, and reply with the translation only.
Target language: {{input}}
When no target language is given: translate into {{my_language}}, unless the text is already in {{my_language}}. Then translate into English, or into Spanish when the text is English.
