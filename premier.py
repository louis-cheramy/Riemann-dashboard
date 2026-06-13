import math
import os
import time

import numpy as np

MAGIC = b"PRIMEV2\x00"
DTYPE = np.dtype("<u8")
SEGMENT_SIZE = 50_000_000


def simple_sieve(limit):
    if limit < 2:
        return np.array([], dtype=DTYPE)
    sieve = np.ones(limit + 1, dtype=np.bool_)
    sieve[:2] = False
    sqrt_limit = int(math.isqrt(limit))
    for i in range(2, sqrt_limit + 1):
        if sieve[i]:
            sieve[i * i :: i] = False
    return np.flatnonzero(sieve).astype(DTYPE)


def crible_segmente(n, segment_size=SEGMENT_SIZE):
    print(f"Recherche de tous les nombres premiers jusqu'à {n:,}...")
    limite_racine = int(math.isqrt(n)) + 1
    premiers_base = simple_sieve(limite_racine)
    output = os.path.abspath("nombres_premiers.bin")
    print(f"Chemin complet du fichier : {output}")

    with open("nombres_premiers.bin", "wb") as fichier:
        fichier.write(MAGIC)
        fichier.write(premiers_base.tobytes())

        debut = limite_racine + 1
        segment_num = 0
        while debut <= n:
            fin = min(debut + segment_size, n + 1)
            segment = np.ones(fin - debut, dtype=np.bool_)

            for p in premiers_base:
                p = int(p)
                start = max(p * p, ((debut + p - 1) // p) * p)
                if start < fin:
                    segment[start - debut :: p] = False

            primes = np.flatnonzero(segment).astype(DTYPE) + debut
            if primes.size:
                fichier.write(primes.tobytes())

            segment_num += 1
            print(f"Segment {segment_num} : {debut:,} -> {fin - 1:,} traite ({primes.size:,} premiers).")
            debut = fin

    print(
        "Tous les nombres premiers ont été enregistrés dans 'nombres_premiers.bin' "
        "(format uint64, 8 octets par entier)."
    )


if __name__ == "__main__":
    try:
        n = int(input("Entrez la borne maximale (ex: 10000000000) : "))
        if n < 2:
            raise ValueError
        start = time.time()
        crible_segmente(n)
        duration = time.time() - start
        print(f"Terminé en {duration / 60:.2f} minutes.")
    except ValueError:
        print("Erreur : entrez un entier >= 2.")
