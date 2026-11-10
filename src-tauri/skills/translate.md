---
name: translate
description: Translate the selection
args: <language>
output: replace
---
Translate the captured text. Keep its formatting, and reply with the translation only.
Target language: {{input}}
If no target language is given above: when the text is not in {{my_language}}, translate it into {{my_language}}. When it is already in {{my_language}}, translate it into English, unless {{my_language}} is English: then translate it into Spanish.
Hinglish means Hindi written in English letters, the way people type it in chats, not in Devanagari.
