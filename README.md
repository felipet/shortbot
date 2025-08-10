<div align=center>
<img src="data/img/ShortBot_logo_main.png" width=100px style="border-radius: 50%"/>
<h1><a href="https://t.me/ibexshortbot">@IbexShortBot</a></h1>
</div>

[![License](https://img.shields.io/github/license/felipet/shortbot?style=flat-square)](https://github.com/felipet/shortbot/blob/main/LICENSE)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/felipet/shortbot/rust.yml?style=flat-square&label=CI%20status)

This is a [Telegram bot](https://core.telegram.org/bots) that helps investors to keep
track of short positions against stock companies that belong to the [Ibex35][ibex35].

The information is notified by mutual funds to the regulator ([CNMV][cnmv]) within a
maximum time period. Short positions (>= 0.5% of the total market capitalization of
a company) must be notified as well as changes on those. This information is public
and available in CNMV's web page. However, the design of that page, and its speed makes
really annoying checking positions.

**That's why I made this bot!**

## Bot's features

- Multilingual support for Spanish and English users.
- Simple check of alive short positions for every stock company of the Ibex35.
- Handle subscriptions to stocks to send messages to users after a short position update.
- Subscriptions to stocks to receive notifications when a short positions is updated.

## Usage

To start using this bot, just search @IbexShortBot in Telegram, or open this
[link](https://t.me/ibexshortbot).

# Development

Before making any commit to the repository, [pre-commit] shall be installed to check
that everything within the commit complies with the style rules of the repository.

Then, a ***git hook*** shall be installed. The hooks for this repository are located
at `.githooks`. These can be copied to `.git/hooks/` or used straight from such
location when telling ***git*** where to look for hooks:

```bash
$ git config core.hooksPath .githooks
```

A pre-push hook is also added to avoid pushing code that doesn't pass tests. If you
really aim to push code that doesn't pass tests for some reason, the following command
can be used:

```bash
$ git push --no-verify <remote> <branch>
```

## Running the bot

Since the bot started to use a **websocket** to communicate with Telegram's Bot API, it is no longer possible to
simply run the bot locally for development.

A local web server is deployed using `Axum`. If the application is deployed without a proxy, opening the chosen
port in the firewall and having a registered domain name would be the only requirements.

### Using a proxy

The current deployment of the application includes a proxy server in between that handles all the requests to
the chosen domain name (Apache).

To enable development deployments, add the following to a new or existing site of the Apache server:

```
# Proxy for the testing webhook with shortbot
ProxyPassReverse /<path> http://127.0.0.1:<port>/<path>
ProxyPass /<path> http://127.0.0.1:<port>/<path>
```

Then, a port forwarding to the development computer from the server, which is hosting the Apache server,
might be needed. If so, simply use `ssh -R` this way:

```bash
$ ssh -N -R <server IP>:<port>:localhost:<port> <user>@<server IP>
```


[ibex35]: https://www.bolsasymercados.es/bme-exchange/es/Mercados-y-Cotizaciones/Acciones/Mercado-Continuo/Precios/ibex-35-ES0SI0000005
[cnmv]: https://www.cnmv.es/portal/home.aspx
