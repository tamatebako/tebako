# Development notes #
## Ruby packaging specification ##

Tebako Ruby packager support 5 different scenarious depending on the configuration files that are present in the root project folder:

| Scenario |\*.gemspec| Gemfile  | \*.gem   |
|:--------:|:--------:|:--------:|:--------:|
| 1        |     No   |   No     |   No     |
| 2        |     No   |   No     |   Yes    |
| 3        |     Yes  |   No     |   Any    |
| 4        |     Yes  |   Yes    |   Any    |
| 5        |     No   |   Yes    |   Any    |


| Scenario |     Description     |      Packaging      |     Entry point     |
|:--------:|:-------------------:|:-------------------:|:-------------------:|
| 1        | Simple ruby script  |                     |                     |
| 2        |   |                     |                     |
| 3        |   |                     |                     |
| 4        |   |                     |                     |
| 5        |   |                     |                     |




