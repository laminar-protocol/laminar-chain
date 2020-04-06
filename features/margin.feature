Feature: Margin Protocol

  Scenario: Margin liquidity pool
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  | Result        |
      | Pool  | $10 000 | Ok            |
      | Alice | $5 000  | Ok            |
      | Alice | $6 000  | BalanceTooLow |
    Then margin liquidity is $15000

  Scenario: Open and close
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $10 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $5 000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin set swap rate
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
    And margin enable trading pair EURUSD
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $5000  |
    When close positions
      | Name  | ID | Price |
      | Alice | 0  | $2    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $4700  |
    And margin liquidity is $10 300
    When margin withdraw
      | Name  | Amount |
      | Alice | $4700  |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $9700 | $0     |

  Scenario: margin trader take profit
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $10 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $5 000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin set swap rate
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
    And margin enable trading pair EURUSD
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $5000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $4     |
    When close positions
      | Name  | ID | Price |
      | Alice | 0  | $2    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $9650  |
    And margin liquidity is $5350

  Scenario: margin trader stop lost
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $10 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $5 000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin set swap rate
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
    And margin enable trading pair EURUSD
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $5000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $2.8   |
    When close positions
      | Name  | ID | Price |
      | Alice | 0  | $2    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $3710  |
    And margin liquidity is $11 290

  Scenario: margin trader liquidate
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $10 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $5 000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin set swap rate
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
    And margin enable trading pair EURUSD
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $5000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $2.2   |
    And margin trader margin call
      | Name  | Result     |
      | Alice | SafeTrader |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $2.1   |
    And margin trader margin call
      | Name  | Result |
      | Alice | Ok     |
    And margin trader liquidate
      | Name  | Result                  |
      | Alice | NotReachedRiskThreshold |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $1.9   |
    And margin trader liquidate
      | Name  | Result |
      | Alice | Ok     |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $-745  |
    Then margin liquidity is $15000

  Scenario: margin liquidity pool liquidate
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $10 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $5 000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin set swap rate
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
    And margin enable trading pair EURUSD
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $5000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $4.1   |
    And margin liquidity pool margin call
      | Result   |
      | SafePool |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $4.2   |
    And margin liquidity pool margin call
      | Result |
      | Ok     |
    And margin liquidity pool liquidate
      | Result                  |
      | NotReachedRiskThreshold |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $5.0   |
    And margin liquidity pool liquidate
      | Result |
      | Ok     |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $14600 |
    Then margin liquidity is $0

  Scenario: margin multiple users multiple currencies
    Given accounts
      | Name  | Amount  |
      | Pool  | $20 000 |
      | Alice | $10 000 |
      | Bob   | $10 000 |
    And margin create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $20 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $9 000  |
      | BOB   | $9 000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
      | FJPY      | $5     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
      | JPYEUR  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
      | JPYEUR | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin set swap rate
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
      | JPYEUR  | -1%  | 1%    |
    And margin enable trading pair EURUSD
    And margin enable trading pair JPYEUR
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
      | BOB   | JPYEUR | Short 10 | $6000  | $1    |
    Then margin balances are
      | Name  | Free  | Margin |
      | Alice | $1000 | $9000  |
      | BOB   | $1000 | $9000  |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3.1   |
      | FJPY      | $4.9   |
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | JPYEUR | Long 20  | $1000  | $4    |
    When close positions
      | Name  | ID | Price |
      | BOB   | 1  | $4    |
    Then margin balances are
      | Name  | Free  | Margin                 |
      | Alice | $1000 | $9000                  |
      | BOB   | $1000 | 9996000000000000008400 |
    And margin liquidity is 19003999999999999991600
    And oracle price
      | Currency  | Price  |
      | FEUR      | $2.9   |
      | FJPY      | $5.1   |
    When close positions
      | Name  | ID | Price |
      | Alice | 0  | $2    |
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | BOB   | EURUSD | Short 20 | $2000  | $2    |
    Then margin balances are
      | Name  | Free  | Margin                 |
      | Alice | $1000 | $8205                  |
      | BOB   | $1000 | 9996000000000000008400 |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $2.8   |
      | FJPY      | $5.2   |
    When close positions
      | Name  | ID | Price |
      | Alice | 2  | $1    |
      | BOB   | 3  | $4    |
    Then margin balances are
      | Name  | Free  | Margin                  |
      | Alice | $1000 | 8882935483870967742000  |
      | BOB   | $1000 | 10082000000000000008400 |
    And margin liquidity is 19035064516129032249600
