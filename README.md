# routedatacollect (Working title)

A data collection program that queries the Google Routes API for a very specific route
and saves it to a database. Uses a custom job scheduler that lets me run time-sensitive
tasks at the right moments.

I intend to run this program on a small server from 6/1/2023 to at least 10/1/2023, then use this
data to help me choose the best time to take the bus home from school.

## Why do this?

I love using Google Maps for its execellent integration with my city's bus system, but
there is this train that passes through a high traffic street that my bus crosses. 
Maps does not report nor factor this train into the bus route's duration until after the train passes
and the resulting traffic data is collected.

I figured that the train has to follow some sort of schedule, even if that schedule is hard-to-find 
or most likely private. Therefore, I built this program to query Google's Routes API for every possible time
that I might leave my school to travel home, in order to collect some historical data that I could use
to better predict when I should leave school to avoid the heavy traffic caused by the train.

## Current testing methodology

My current way to test this is not great or sound by any means. Nevertheless, here is the rough idea
- Every hour at the 15th minute mark, a bus leaves my school and the bus route begins. Let's use 1:15pm as an example:
  - 1:15pm: Query Routes for the travel duration from the school to my destination.
  - 1:42pm: Query Routes for the travel duration from where the bus would be at 1:42pm to my destination.
  - 2:25pm: Query Routes for the travel duration from where the bus would be at 2:25pm to my destination.
  - 2:38pm: Query Routes for the travel duration from where the bus would be at 2:38pm to my destination.
  - 2:45pm: This is the soonest the bus can arrive at my destination. From 2:38pm to at least here, the bus 
    reaches or passes the train tracks.
- Repeat this process every hour, 5:15pm being the last time to run this process for the day
- We're hopefully going to see very similar travel durations for each step of the way, except for when we
  see fairly high traffic, which may or may not be the train. What's important is that if we look at this data
  over a large spread of time, the anomolies should pop up pretty well, and (hopefully) resemble some pattern.

## Results (check back in October, 2023!)

## Looking to the future

While there may already be dozens of tools like this, it'd be very cool if I could generalize this program
to track any set of locations that experience very high, but regular, levels of traffic due to something like a train line.

Also, there's definitely a much more efficient way to do this, and it's possible I find this out somewhere between 6/1 and 10/1. 
If that is the case, and it renders my data unusable, then this was still a super fun project to write. This really
tested my skills and problem-solving abilities as a budding developer, and made use of many different software 
development practices and tools that I had little experience with, such as concurrent and asynchronous programming, databases, docker containers, 
serialization, data structures, and error handling. 
