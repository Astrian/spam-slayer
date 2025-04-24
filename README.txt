A Telegram-oriented channel discusses group spam detectors and processors. Currently will:

- Scan all the text messages
- Handover all messages to Gemini, then require it to judge if the message is spam or not
- Delete the messages judged as spam
- Preserve all messages with judge result

Make @commSpamSlayerBot your discussion group’s admin and grant the “delete messages” permission.
We recommend using this bot with group chats only for channel comments; it is not guaranteed for
general group chats.

📌 Data Usage Notice: All text messages received by the bot will be stored, including sender name, 
avatar, actions, and comment content. These data will solely be used for training and improving 
the classification model, and will not be used for any other purposes.

License: MIT
