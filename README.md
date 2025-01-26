# hubble

hubble exposes the [IMDb datasets](https://developer.imdb.com/non-commercial-datasets/) as a GraphQL API.
It will auto-update when new datasets are released.
Why the name? Uhhhh....

## setup

> [!CAUTION]
> The IMDb datasets are intended for personal and non-commercial use only. Make sure you comply with the licensing terms listed [here](https://developer.imdb.com/non-commercial-datasets/).


`/data` is used to store the downloaded datasets, the database, and the search index.
`/data` will use around 5GB of space.
hubble will download about 1GB of data every day.

```bash
docker run --rm -p 8000:8000 -v hubble-data:/data sylver/hubble
```

```yaml
services:
  hubble:
    container_name: hubble
    image: sylver/hubble
    ports:
      - 8000:8000
    volumes:
      - hubble-data:/data

volumes:
    hubble-data: {}
```

## usage

GraphiQL is available at `http://localhost:8000`.
You can poke around the schema and run queries, for example:

```graphql
query {
  titles(query: "the expanse", limit: 5) { 
    id
    kind
    primaryTitle
    rank
  }
}
```

<details>

<summary>Response</summary>

```json
{
  "data": {
    "titles": [
      {
        "id": "tt3230854",
        "kind": "TV_SERIES",
        "primaryTitle": "The Expanse",
        "rank": 19.19731330871582
      },
      {
        "id": "tt13845484",
        "kind": "TV_SERIES",
        "primaryTitle": "The Expanse Aftershow",
        "rank": 16.741209030151367
      },
      {
        "id": "tt16442600",
        "kind": "TV_SERIES",
        "primaryTitle": "The Expanse: One Ship",
        "rank": 14.8692626953125
      },
      {
        "id": "tt0069730",
        "kind": "MOVIE",
        "primaryTitle": "The Weapon, the Hour, the Motive",
        "rank": 4.546613693237305
      },
      {
        "id": "tt0094500",
        "kind": "TV_MINI_SERIES",
        "primaryTitle": "The Lion, the Witch & the Wardrobe",
        "rank": 4.546613693237305
      }
    ]
  }
}
```

</details>

```graphql
{
  title(id: "tt2560140") {
    id
    kind
    primaryTitle
    originalTitle
    startYear
    endYear
    isAdult
    runtimeMinutes
    genres
    rating {
      numVotes
      averageRating
    }
    akas {
      title
      region
      language
      types
      attributes
    }
    episodes {
      id
      seasonNumber
      episodeNumber
      title {
        id
        kind
        primaryTitle
        rating {
          numVotes
          averageRating
        }
      }
    }
  }
}
```

<details>

<summary>Response</summary>

```json
{
  "data": {
    "title": {
      "id": "tt2560140",
      "kind": "TV_SERIES",
      "primaryTitle": "Attack on Titan",
      "originalTitle": "Shingeki no Kyojin",
      "startYear": 2013,
      "endYear": 2023,
      "isAdult": false,
      "runtimeMinutes": 24,
      "genres": [
        "Action",
        "Adventure",
        "Animation"
      ],
      "rating": {
        "numVotes": 579617,
        "averageRating": 9.100000381469727
      },
      "akas": [
        {
          "title": "Shingeki no Kyojin",
          "region": null,
          "language": null,
          "types": [
            "original"
          ],
          "attributes": []
        },
        {
          "title": "Ataque dos Titãs",
          "region": "BR",
          "language": null,
          "types": [
            "imdbDisplay"
          ],
          "attributes": []
        },
        ...
      ],
      "episodes": [
        {
          "id": "tt2825724",
          "seasonNumber": 1,
          "episodeNumber": 1,
          "title": {
            "id": "tt2825724",
            "kind": "TV_EPISODE",
            "primaryTitle": "To You, in 2000 Years: The Fall of Shiganshina, Part 1",
            "rating": {
              "numVotes": 37753,
              "averageRating": 9.100000381469727
            }
          }
        },
        {
          "id": "tt2844574",
          "seasonNumber": 1,
          "episodeNumber": 2,
          "title": {
            "id": "tt2844574",
            "kind": "TV_EPISODE",
            "primaryTitle": "That Day: The Fall of Shiganshina, Part 2",
            "rating": {
              "numVotes": 27836,
              "averageRating": 8.5
            }
          }
        },
        ...
      ]
    }
  }
}
```

</details>

## todo

- [ ] Entries already in the search index are ignored, but that means vote scores are not updated, which may affect search significantly over time.
- [ ] Use more dataloaders
- [ ] If an error occurs during imports, the process is not retried until the dataset updates.