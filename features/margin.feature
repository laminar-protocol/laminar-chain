Feature: Margin Protocol

  Scenario: Margin liquidity pool
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And create liquidity pool
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
    And create liquidity pool
    And margin deposit liquidity
      | Name  | Amount  |
      | Pool  | $10 000 |
    And margin deposit
      | Name  | Amount  |
      | Alice | $5 000  |
    And oracle price
      | Currency  | Price  |
      | AUSD      | $1     |
      | FEUR      | $3     |
    And margin spread
      | Pair    | Value |
      | EURUSD  | 1%    |
    And margin set accumulate
      | Pair   | Frequency | Offset |
      | EURUSD | 10        | 1      |
    And margin set min leveraged amount to $100
    And margin set default min leveraged amount to $100
    And margin update swap
<<<<<<< HEAD
      | Pair    | Long | Short |
      | EURUSD  | -1%  | 1%    |
=======
      | Pair    | Value |
      | EURUSD  | 1%    |
>>>>>>> add cucumber tests
    And margin enable trading pair EURUSD
    When open positions
      | Name  | Pair   | Leverage | Amount | Price |
      | Alice | EURUSD | Long 10  | $5000  | $4    |
    Then balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $5000  |
    When close positions
      | Name  | ID | Price |
      | Alice | 0  | $2     |
    Then balances are
      | Name  | Free  | Margin |
      | Alice | $5000 | $4700  |
    And margin liquidity is $10 300
    When withdraw
      | Name  | Amount |
      | Alice | $4700  |
    Then balances are
      | Name  | Free  | Margin |
      | Alice | $9700 | $0     |
