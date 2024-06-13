![GitHub License](https://img.shields.io/github/license/felipet/shortbot)
![GitHub Release](https://img.shields.io/github/v/release/felipet/shortbot)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/felipet/shortbot/rust.yml?branch=development&style=flat&label=CI%20status&link=https%3A%2F%2Fgithub.com%2Ffelipet%2Fshortbot%2Factions%2Fworkflows%2Frust.yml)
![Last Release](https://img.shields.io/github/v/tag/felipet/shortbot?include_prereleases)



<div align=center>
<img src="data/img/ShortBot_logo_main.png" width=100px style="border-radius: 50%"/>
<h1><a href="https://t.me/ibexshortbot">@IbexShortBot</a></h1>
</div>


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

## Usage

To start using this bot, just search @IbexShortBot in Telegram, or open this
[link](https://t.me/ibexshortbot).


[ibex35]: https://www.bolsasymercados.es/bme-exchange/es/Mercados-y-Cotizaciones/Acciones/Mercado-Continuo/Precios/ibex-35-ES0SI0000005
[cnmv]: https://www.cnmv.es/portal/home.aspx